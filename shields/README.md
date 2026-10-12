# shields — `Shields.asi`

When a weapon strikes a ship whose shields hold, stock flashes a shell on the shield
where it lands: a flat, saturated green cup (red as the shields run out) that pops up
and is gone. `Shields.asi` draws those hits itself, in a shader over the ship's whole
shield ellipsoid, which stays invisible except about each hit: a hot spot where it
struck with a white core and a flash at the blow, a ring running out across the shield
from there, a soft glow over the shield and its rim (where the shell is seen edge on) lit
around the hit and fading, all coloured by the struck ship's faction, weaker and
flickering as its shields run out. Up to eight hits a ship at once.

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
  whose model is type 0's is skipped while the plugin draws; every other type (the
  shields-down ring, the special weapons' shells) calls through and is drawn by stock.
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

## Shields.ini

`Enable`, `Reload`, `LogHits`, `Log`; `Brightness`; `ColourMode`, `Federation`,
`Klingon`, `Romulan`, `Borg`, `Cardassian`, `Species8472`, `Other`, `Colour`,
`LowColour`, `LowShift`; `Spot`, `SpotSize`, `Core`, `Flash`, `FlashTime`, `Duration`,
`Sustain`, `FadeOut`; `Ring`, `RingSpeed`, `RingWidth`, `RingTime`, `RingPeriod`,
`RingLattice`; `Lattice`, `Rim`, `RimPower`, `GlowReach`, `GlowTime`, `GlowSustain`;
`Pattern`, `HexCells`, `HexLine`, `Crackle`; `DamageRef`, `StrengthMin`,
`StrengthMax`; `WeakLevel`, `Flicker`, `FlickerBelow`, `FlickerRate`; `TimeScale`. Each
is described in the file.

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
  dimmer and greyer, and once down the stock shields-down ring took over (type 1, left
  stock) and the beams reached the hull.
- Under steady fire from two ships the shield is lit over much of its
  underside; with `GlowReach=0.7` (the first default) the whole shell glowed faintly
  blue, so it is 0.35.

`./a2test run shields-hits --install .` checks the hooks and that both kinds of hit are
drawn, and keeps three shots; `no-assets` checks it installs and patches without assets.

## Not covered yet

- The release package (`publish/`, the GUI installer) does not carry the layer.
- Only type 0, the ordinary hit, is drawn by the shader; the shields-down ring and the
  special weapons' shells stay stock.
- Hits on objects that are not craft (no shields at +0x1c8) are drawn at full strength.
- The flicker below `FlickerBelow` has not been judged frame by frame; the bench run
  above saw the shields fall too fast to catch it.
- Not yet seen in game, nor in a busy fleet battle; the cost per struck ship (one
  1,224-triangle draw, a pixel shader over its ellipsoid) has not been measured.
