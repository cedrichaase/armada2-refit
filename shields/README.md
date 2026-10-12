# shields — `Shields.asi`

When a weapon strikes a ship whose shields hold, stock flashes a shell on the shield
where it lands: a flat, saturated green cup (red as the shields run out) that pops up
and is gone. `Shields.asi` draws those hits itself, in a shader over the ship's whole
shield ellipsoid, which stays invisible except about each hit: a hot spot where it
struck with a white core and a flash at the blow, a ring running out across the shield
from there, a soft glow over the shield and its rim (where the shell is seen edge on) lit
around the hit and fading, all coloured by the struck ship's faction, weaker and
flickering as its shields run out. Up to eight hits a ship at once.

When a ship's shields are down, stock circles it with a band of lightning texture.
`Shields.asi` draws that too ("Shields down", below): jagged blue-white arcs crackling
over the ship's shield ellipsoid in discharges that flare and die where they strike,
mostly about its equator, struck afresh several times a second so that nothing turns;
a flash over the shell as the shields collapse, and a flickering light on the ship's own
hull from the brightest discharge.

It changes what is drawn and nothing else: the hits are the engine's, made, moved and
timed by its own weapons, and nothing the simulation reads is touched, so a player with
it can play a player without it.

```
shields/install.sh            build, then install Shields.asi and Shields.ini
shields/install.sh --remove   take them out again
```

`./install` runs it; `./a2mod` switches it as the `shields` layer. It needs nothing from
`A2_DATA`: the effect is procedural. Without a Direct3D 9 device behind d3d8 (DXVK's own
d3d8, `platform/D3D9.md`) or with `Enable=0`, the game draws its own shield hits.

## How stock draws a shield hit

Addresses from `armada2.map`, GOG patch 1.1. Established by reading `Armada2.exe`
for the hooks below, and on the bench.

- **`ShieldEffect`** is a `GameFeature` (the static `shieldEffect`, vtable 0x6b0134).
  Its `PostLoad` loads one model per **shield type** into `m_logical_db` /
  `m_visible_db` (0x736ddc / 0x736dd8), eight of them, named in a table of strings:
  type 0 `xshldx01` (the ordinary hit), 1 `xshlddie` (the shields-down ring), then the
  special weapons' shells (`wshldadd`, `wshldsub`, `wcorbrfr`, `wshldrmd`, `wmanhiem`,
  `WEclairlink1`). A weapon's type comes from its ODF's `shieldGeometryName` (default
  the first, `GetDefaultSOD`), its duration from `shieldDuration`
  (`OrdnanceClass`+0x120 and +0x124).
- **`SOD/Xshldx01.SOD`** is a shell (mesh `shield3`, material `additive`) textured with
  `Xshldx01.tga`: a 4x4 flipbook of a grey ring growing outwards and breaking up.
- **`ShieldEffect::CreateShieldHit(GameObject *, const Matrix34 &, ShieldType, float,
  int)`** (0x4743b0, static, cdecl) makes a `ShieldHit` (an `ST3D_Instance`, 0xc4
  bytes) of that type's model and lists it in `m_shieldList` (0x736de8, a
  `std::map<int, auto_ptr<ShieldHit>>` keyed by an id that only grows), returning the
  id, or -1 when the type has no model. The weapons call it from four sites after they
  deal their damage (`Beam::Simulate` 0x58bb95, `Bullet::Simulate` 0x58cad8,
  `Mine::mMoveTowardsTarget` 0x58d502, `Missile::Simulate` 0x58dd65); the ordnance is
  in `ebx` at every one. A beam keeps the id (+0xe8) and moves the hit along with it
  through `ShieldEffect::ShieldUpdate` (0x4747d0) every tick.
- **The matrix the weapons pass** comes from `Ordnance::Collision`: the impact point is
  taken into **shield space**, where the shield is the unit sphere (the class's
  object-to-shield transform, `GameObjectClass`+0x8, and its inverse, shield-to-object,
  at +0x38), and `BuildDirectionalMatrix` turns a frame toward it: row 2 is the
  direction of the hit, the position zero.
- **`ShieldHit::ShieldUpdate`** (0x473d50) composes that frame with the shield-to-object
  transform whose diagonal is divided by the model's radius (+0x40), normalises the
  rows into the instance's transform (+0x90) and keeps their lengths as its scale
  (+0x28). So the model's pole points at the hit and the shell is stretched onto the
  shield ellipsoid. It also notes the struck craft's shields (`Craft::GetShields`,
  0..1) at +0xc0; +0x84 is the time left, +0x88 whether it runs out, +0x8c the target's
  handle.
- **`ShieldEffect::Simulate`** counts each timed hit down and erases it at zero;
  **`ShieldEffect::Render`** (0x474910, called from `GameFeature_RenderAll` in
  `Armada_RenderAllOurStuff`) calls **`ShieldHit::DrawShield`** (0x474070) for each.
  That skips a target the player cannot see (its `CanUserSee`), sets the shield
  lighting material's colour to (1 − s, s, 0) for the shields s noted at the hit (white
  for none), puts it on the engine as an override material, and renders the instance
  under the object's transform. It is blended, so it goes the CPU path and is sorted
  and drawn at the device's `Flush`.

On the bench (`SCENE=shields` and `SCENE=firing`, `Enable=0`), that is a flat green
cup on the struck side of the ship, opaque-looking and the same green on every
faction.

## The hooks

Every site is checked before it is written (`CreateShieldHit`'s and `DrawShield`'s
first bytes, the list's head read in `ShieldEffect::Render`, and a `call` at each call
site); a different `Armada2.exe` leaves the plugin inert and says so in `Shields.log`.
**Every hook is a call site, chained**: the plugin calls whatever the call went to when
it patched it, so another plugin's hook at the same site keeps working, and the log
names a target that is not the engine's.

- **The call to `ShieldHit::DrawShield` at 0x47492d in `ShieldEffect::Render`.** A hit
  whose model is type 0's, or type 1's (the shields-down ring) with `Down=1`, is
  skipped while the plugin draws; every other type (the special weapons' shells) calls
  through and is drawn by stock.
- **The call to `ST3D_GraphicsEngine::RenderChunks` at 0x598446 in
  `Armada_RenderAllOurStuff`**: after the objects and `GameFeature_RenderAll`, before
  `RenderParticleList` (which `Nebulae.asi` hooks, a different site). The plugin calls
  it, then draws the hits: every opaque hull is in the depth buffer by then. The
  function runs once per view, so the HUD's action camera gets its own draw from its
  own camera (the device's `VIEW` and `PROJECTION`); nothing the plugin keeps from
  frame to frame depends on the view.
- **The four weapon calls of `CreateShieldHit`.** Each goes through a thunk that notes
  the ordnance (`ebx`) and the site, then jumps to a wrapper with the call's own
  arguments, which calls on and records the id it returns against the weapon class's
  damage: `damageBase`, the default of the `cLookup` at `OrdnanceClass`+0xa8 (+0x14 in
  it). `testbench/scene`'s `ShieldFx=0` rewrites the same four calls; it saves and
  restores whatever is there, so the two coexist.
- **The call to `Ordnance::PreRenderAll` at 0x598199**, right after
  `GameObject_PreRenderAll`: the plugin calls it, then registers the shields-down
  lights. `Repair.asi` chains the same call.
- **The d3d8 device's `Reset`** (slot 14), chained as `Nebulae.asi` and `Lighting.asi`
  chain it: the state block is let go.

## The draw

Per frame, the plugin walks `m_shieldList` (head at +4 of the map, a node's key at
+0xc and its `ShieldHit` at +0x14, `_Nil` at 0x736e38, in order as the engine walks it)
and keeps its own record per id of the type-0 hits: born when first seen, dying when
the engine erases it, and kept until its fade is over. The hit's direction in shield
space is recovered from its instance transform: that is H S′ (H the hit's frame, S′
the shield-to-object 3x3 with its diagonal over the model's radius), so row 2 of H is
row 2 of the transform, times its scale, times S′⁻¹. That follows a beam as
`ShieldUpdate` moves it.

Per struck ship (by handle, skipped if the player cannot see it, as `DrawShield` asks),
one draw: a unit sphere of 1,224 triangles mapped by the shield-to-object transform,
the instance's scale (when it is scaled) and the object's world transform onto the
shield ellipsoid; additive, depth-tested against the hulls, no depth write, no culling
(a hit on the far side shows through the shell where no hull hides it). `shields.hlsl`
does the rest per pixel, with distances in shield radii (the cube root of the
ellipsoid's volume factor), so a scout and a station look alike:

- **the spot**: a Gaussian about the hit. A shot (torpedo, pulse, mine, missile) is one
  blow: a flash, then a fade over `Duration`. A beam holds it at `Sustain` while it plays
  on the shield, then fades over `FadeOut`. Its core goes white (`Core`).
- **the ring**: runs out at `RingSpeed` from the blow, `RingWidth` wide, gone after
  `RingTime`; a beam sends one every `RingPeriod` while it plays. It can show the
  lattice (`RingLattice`, 0 by default: a smooth ring).
- **the field**: a smooth glow (`Pattern=none`, the default) or a lattice (`hex`: hexes
  on the three planes of the unit sphere, blended by how squarely each faces the point;
  `cells`: a random brightness per cell), and the rim (`(1 − |N·V|)^RimPower`), lit
  within `GlowReach` of the hit and over `GlowTime`. The hex lattice was the first
  default and was dropped before release (2026-10-12): the user did not want a pattern
  on the shields. With `none` the glow is even where the hex fill averaged about 0.4,
  so `Lattice` went from 0.6 to 0.25 to keep the shell about as bright, and
  `RingLattice` from 0.6 to 0.
- **strength**: `sqrt(damage / DamageRef)` within `StrengthMin..StrengthMax`, from the
  weapon's `damageBase` (a Galaxy's phasers 100–110, a Klingon battleship's torpedoes 55
  and 100 on the bench).
- **colour**: `ColourMode=faction`, by the struck ship's ODF prefix (`f`, `k`, `r`, `b`,
  `c`, `8472`; anything else `Other=`); `stock`, (1 − s, s, 0) as stock tints its shell;
  `fixed`, `Colour=`. As the shields run out (the craft's +0x1c8 over +0x1cc, read live)
  the colour moves towards `LowColour` by `LowShift`, the brightness falls towards
  `WeakLevel`, and below `FlickerBelow` it flickers `FlickerRate` times a second, up to
  `Flicker` deep; the lattice crackles more.

The clock is wall time (`TimeScale=` slows it down, for looking at the effect frame by
frame), so the effect plays on while the game is paused.

## Shields down

**What stock does.** A craft whose shields fail gets a hit of type 1 (`xshlddie`): an
untimed `ShieldHit` made by `Craft::ShieldsDown` (0x4c74f0, which keeps its id at
craft+0x208) with an identity frame, so the shell is the whole shield ellipsoid; several
other `Craft` paths (`DamageAlloc`, `ShieldCollapse`, `CheckDerelict`, the system set-up)
make the same type. It stays in `m_shieldList` for as long as the shields are down.
`SOD/Xshlddie.SOD` is a band about the ship's middle, textured with `Xshlddie.tga`: four
frames of blue lightning with white cores, turning as it plays. On the bench
(`SCENE=shields`, `heal target off`, 2026-10-12) that is a saturated blue ring of
lightning cutting through the hull, the same on every ship.

**The draw.** The walk that finds the ordinary hits keeps one record per ship that has
a type-1 hit (born when it first appears, dying when it goes, then faded out over
`DownFadeOut`), and on it up to six *discharges*: a place on the shell in shield space,
chosen at random with its height cubed so that most fall about the equator (where stock's
band is), lit for `DownStrikeMin..Max` seconds, rising in 0.03 s and falling with the
square of the time left, a new brightness 30 times a second; then it strikes again
elsewhere after up to twice `DownStrikeGap`. Per ship, one more draw over the same
sphere, after the hits, with its own pixel shader (`down_ps`):

- **arcs**: the zero lines of a 3D gradient noise on the unit sphere (`DownScale` cells
  across it), its domain warped twice, coarse and fine (`DownWarp`), so they run jagged.
  The core is a line of constant pixel width (`DownCore`, from the noise's screen-space
  derivative), so it stays sharp and readable at every distance; a halo of four times
  that, and a glow in the noise's own units (`DownGlowWidth`). Only stretches of each
  line are lit (`DownLit`), so they read as separate bolts rather than closed loops;
  forks are a finer field's zero lines, shown only near a main arc (`DownBranches`).
- **where**: the arcs show inside the discharges (a Gaussian `DownStrikeSize` shield
  radii wide about each) and faintly all about the equator (`DownCrackle`, a band
  `DownBand` high).
- **no turning**: two fields of arcs, each struck afresh (a new seed) as its share
  passes zero, half a step apart, `DownRate` steps a second. The arcs jump from place to
  place and the whole shell never pops at once. Value noise was not used: through a
  threshold its grid shows (`nebulae/README.md`); gradient noise with a quintic fade
  has none.
- **the collapse**: a flash over the shell (`DownFlash`, brightest at the rim) fading
  over `DownFlashTime` as the shields go.
- **colour**: a white-hot core (`DownArc`) in a blue glow (`DownGlow`), as stock's
  texture, on every faction; the far side of the shell, seen through it, at
  `DownFarSide`; a faint rim haze (`DownHaze`).

The first build lit the zero lines everywhere at a pixel's width: smooth contour loops
over the whole shell, like a marbled glass dome. The bolts came from the second, finer
warp, `DownLit` and the brighter discharges.

**The light.** Per ship, its brightest discharge (plus the collapse flash) registers an
ordinary `ST3D_Point_Light` with the engine at its place on the shell, after
`GameObject_PreRenderAll` (the chained call to `Ordnance::PreRenderAll`):
`DownLightColour` x `DownLight` x its brightness, full to `DownLightReach` of the ship's
bounding radius and gone a fifth of that further on, which `Lighting.asi`'s
`pick_points` takes as a hard light. The reach is the bounding radius's: the shield's
mean radius on a flat ship is small (27 units on a Galaxy against a bounding radius of
48), and at 0.9 of it the light reached almost none of the hull. One light object per
ship, up to eight, made on first use as `Lighting.asi` makes its own.

On the bench (2026-10-12, 1920x1080, `orbit target 150 -25 150`, the damaged Galaxy
from below, a 900x420 box over its saucer's underside, mean RGB of five frames each):
`DownLight=0` 23.2/29.8/41.3, `DownLight=4` 27.7/35.6/48.5, with blue patches on the
underside under the discharges. From above, where the Key already lights the saucer near
white, it was within frame-to-frame noise at 1.5 and at 4.

## Shields.ini

`Enable`, `Reload`, `LogHits`, `Log`; `Brightness`; `ColourMode`, `Federation`,
`Klingon`, `Romulan`, `Borg`, `Cardassian`, `Species8472`, `Other`, `Colour`,
`LowColour`, `LowShift`; `Spot`, `SpotSize`, `Core`, `Flash`, `FlashTime`, `Duration`,
`Sustain`, `FadeOut`; `Ring`, `RingSpeed`, `RingWidth`, `RingTime`, `RingPeriod`,
`RingLattice`; `Lattice`, `Rim`, `RimPower`, `GlowReach`, `GlowTime`, `GlowSustain`;
`Pattern`, `HexCells`, `HexLine`, `Crackle`; `DamageRef`, `StrengthMin`,
`StrengthMax`; `WeakLevel`, `Flicker`, `FlickerBelow`, `FlickerRate`; `TimeScale`;
`Down`, `DownBrightness`, `DownArc`, `DownGlow`, `DownHaze`, `DownScale`, `DownWarp`,
`DownCore`, `DownGlowWidth`, `DownBranches`, `DownLit`, `DownRate`, `DownStrikes`,
`DownStrikeSize`, `DownStrikeMin`, `DownStrikeMax`, `DownStrikeGap`, `DownCrackle`,
`DownBand`, `DownFlash`, `DownFlashTime`, `DownFadeOut`, `DownFarSide`, `DownRimPower`,
`DownLight`, `DownLightColour`, `DownLightReach`. Each is described in the file.

## On the bench

`SCENE=shields` (`testbench/scene/scenes/shields.ini`): a Galaxy (phasers) and a Klingon
battleship (torpedoes) firing at a second Galaxy, healed so its shields hold. 1920x1080,
`orbit target 160 20 170`, 2026-10-12:

- `Shields.log`: every hook patched, all four weapon sites; hits logged by beam
  (damage 100/110, strength 1.29/1.35) and missile (55/100, 0.96/1.29), their
  directions unit vectors in shield space; the first draw at 1,224 triangles.
- Stock (`Enable=0`): a flat saturated green cup at each hit, the same on the Borg cube
  (`SCENE=firing`). With the plugin: blue spots with white cores where the beams and
  torpedoes land, the shield lit around them on the ellipsoid, the beams ending on
  its surface; on the cube, green (Borg) on a sphere about it. (Seen with the hex
  lattice; re-run with `Pattern=none`: smooth glows and rings, the shell no brighter.)
- With `heal target off`, the shields ran out within 20 s; on the way the hits went
  dimmer and greyer, and once down the shields-down ring took over (stock's then; the
  plugin's arcs since 1.1.0) and the beams reached the hull.
- Under steady fire from two ships the shield is lit over much of its
  underside; with `GlowReach=0.7` (the first default) the whole shell glowed faintly
  blue, so it is 0.35.

`./a2test run shields-hits --install .` checks the hooks and that both kinds of hit are
drawn, and keeps three shots; `shields-down-repair` that the shields-down arcs and their
light are drawn once the target's shields run out, with shots close, from below and from
RTS distance; `no-assets` checks it installs and patches without assets.

## Not covered yet

- The release package (`publish/`, the GUI installer) does not carry the layer.
- Only types 0 and 1, the ordinary hit and the shields-down ring, are drawn by the
  shaders; the special weapons' shells stay stock.
- The shields-down arcs cost a second pixel shader over the ellipsoid of every ship whose
  shields are down (two fields of arcs, about twenty noise lookups a pixel); not yet
  measured in a fleet battle.
- Hits on objects that are not craft (no shields at +0x1c8) are drawn at full strength.
- The flicker below `FlickerBelow` has not been judged frame by frame; the bench run
  above saw the shields fall too fast to catch it.
- Not yet seen in game, nor in a busy fleet battle; the cost per struck ship (one
  1,224-triangle draw, a pixel shader over its ellipsoid) has not been measured.
