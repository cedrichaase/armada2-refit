# explosions — `Explosions.asi`

A ship or station that dies goes up in a `FireballExplosion` (the `xfireb*` ODFs,
`classLabel = "fireballexplode"`), and a big one is followed by a `ShockwaveExplosion`
(`xshock`). Stock draws the fireball as a model, `xplode.sod`: eight additive billboards
(`critical1..8` in `Sprites/explosion.spr`) each playing a 4x4 flipbook of an orange
puff. The shockwave is a flat textured disc that grows to ten times the ship's radius.
`Explosions.asi` draws each fireball instead as a volume of gas, ray-marched per pixel
through a sphere, and each shockwave as a thin ring of light. It changes what is drawn
and nothing else: where an explosion is, how long it lasts and what it damages stay the
engine's, so a game against players without it is unaffected.

It needs a Direct3D 9 device behind d3d8 (the d3d8to9 chain, `platform/README.md`).
Without one, or with `Enable=0`, every explosion is stock. It needs no assets: the noise
it draws from is made at start-up.

```
explosions/install.sh            build, then install Explosions.asi and Explosions.ini
explosions/install.sh --timing   ... and log the explosions' GPU time (Timing=1)
explosions/install.sh --remove   take it out again
```

`./install` runs it; `./a2mod` switches it as the `explosions` layer.

## The hooks

Addresses from `armada2.map`, GOG patch 1.1. Every site is checked before it is
written; a different `Armada2.exe` leaves the plugin inert and says so in
`Explosions.log`.

- **Slot 15 of the FireballExplosion vtable (0x6afbd4), `FireballExplosion::Submit`
  (0x4656d0).** `ExplosionFeature::Render` (0x464ba0) calls it for every explosion the
  player can see, once per view. The plugin notes the fireball and does not submit the
  model. The vtable is recognised by slot 13 (`FireballExplosion::Init`, 0x465470), and
  whatever slot 15 holds is what the hook calls through to.
  `Lighting.asi` wraps slots 14 (`Simulate`) and 0 (the destructor) of the same vtable
  for its explosion light, and patches them only while they still hold stock's pointers.
  The ASI loader loads `Explosions.asi` first, so this plugin never touches those two
  slots; it does not need to, since everything it draws is noted at `Submit`.
- **Slot 15 of the ShockwaveExplosion vtable (0x6afdac), `ShockwaveExplosion::Submit`
  (0x4676c0)**, recognised by slot 13 (`ShockwaveExplosion::Init`, 0x466fd0): the same,
  for the rings (`Rings=1`). `OmegaExplosion` has its own vtable and stays stock.
- **The call to `ST3D_GraphicsEngine::RenderChunks` (0x62d470) at 0x598446 in
  `Armada_RenderAllOurStuff`.** Every opaque hull is in the depth buffer after it.
  The plugin calls through to whatever the call went to, then draws what was noted in
  this pass and forgets it. The next call, `RenderParticleList` at 0x598455, is
  `Nebulae.asi`'s, and the device's `Flush` after it `Lighting.asi`'s, so the plugin
  takes neither. Drawing per pass is what makes the HUD's action camera, which draws the
  scene a second time each frame, come out right: each view notes and draws its own.
- **The d3d8 device's `Reset`** (slot 14), chained as `Nebulae.asi` and `Lighting.asi`
  chain it: the state block and the queries are let go first.

What it reads: `Explosion::explosionList` (0x7367c4, a `std::list<Explosion *> *`), to
draw a noted explosion only while it is still alive; an explosion's class (+0x30) and
the radius it was built with (+0x34: `Craft::Explode` builds the fireball with the dead
craft's bounding radius); its transform through `Entity::GetTransform` (0x4cfd50; the
position at +0x24, the up axis at +0xc). A fireball's time left is at +0xa8, counted
down by `FireballExplosion::Simulate`, and its class's `length` at +0x4c. A shockwave's
age is at +0x4c and the radius it stops at at +0x50 (the craft's radius times
`radiusFactor`); `ShockwaveExplosion::GetCurrentRadius` (0x467730) gives its radius
now, and its class holds `color` at +0xd0 and `fadeFactor` at +0xe0. Stock fades the
disc once its radius passes `fadeFactor` of the end, linearly to nothing; the ring
fades the same way.

The time comes from the explosion itself, so it advances per frame (measured on the
bench: the fireball's age moved 16–20 ms a frame at 60 frames a second) and stops when
the game is paused. Nothing needed smoothing.

## The fireball

Drawn on one quad that covers it on screen: a billboard through its centre, widened
for perspective, or the near plane when the camera is close. The pixel shader finds its
own ray, intersects the sphere, and takes 28 steps through it front to back (vkd3d
unrolls every loop and has no `tex3Dlod`, so the count is fixed and the noise texture
has one level: an implicit level inside the unrolled branches cannot matter).

With `s` the fraction of its life gone:

- **Size**: from a fifth of its full radius (`Size=` times the craft's bounding radius,
  no less than `MinRadius=`) to all of it over the first `Growth=` of its life, eased
  out, then swelling a little more.
- **Shape**: the density falls off from the centre and is pushed in and out by the
  noise, which is read in the gas's own coordinates (so its billows grow with it) and
  drifts as it ages; `Turbulence=` scales the push, more as it ages.
- **Heat**: hot through the first half of its life, then cooling (`Cooling=`), and
  hotter towards the centre. The colour runs deep red, orange, yellow, and white only for
  the hottest gas, with the light rising with the square of the temperature.
- **Smoke**: from about a third of its life, gas that has cooled absorbs (`Smoke=`), lit
  faintly by the fire inside it, and all of it thins away over the last part of its life.
  `Opacity=` is how much the hot gas itself hides what is behind it.
- **Flash and halo**: a white flash at the centre, gone in about a fifth of a second
  (`Flash=`), and a soft halo of the fire's light about it while it burns (`Glow=`).
- **Brightness**: the summed light is saturated by its brightest channel, so bright gas
  keeps its hue and only the hottest is white.

Each explosion is seeded from its position: its offset into the noise, so no two look
alike, and the same explosion looks the same from every view.

### Depth

The shader writes its own depth (and the draw writes none): the light-weighted mean
distance along the ray, or the ray's nearest approach to the centre where there is only
the halo. So the hardware's test applies where the light comes from: a hull in front of
the fire hides it, debris inside it is hidden only by what is in front. Translucent
hulls (a fading or cloaking craft) are drawn sorted at the device's `Flush`, after the
fire, and lie over it.

### The noise has no lattice

The same spectral noise as `Nebulae.asi`'s (`nebulae/README.md`): random phases over a
1/f-like spectrum through an inverse FFT, so it tiles and has no grid, 64³ by default
(`NoiseSize=`), made at start-up in about 65 ms. The red channel is the fractal field;
green, blue and alpha are soft fields the shader warps the domain with.

### Tuning, measured

The look was judged on the bench against stock frozen at the same instant: the
`explosion` scene, a ship killed, the game paused, a shot, then `Enable=0` (`Reload=1`
picks it up) and a shot of the stock sprites in the same frame. Two dead ends shaped it:

- **A large warp drew streaks.** Displacing the domain by a quarter of the noise's tile
  sheared the billows into parallel streaks across the fireball. A warp of a few
  hundredths keeps them round.
- **Milky beige at mid-life was dim white light behind smoke**, not the colour ramp:
  measured in a shot, the gas's green and blue stood at 0.64 and 0.32 of its red, the
  ratio of the smoke's own colour, while stock's fire was at 0.48 and 0.14. Starting
  the smoke later, letting it absorb less and keeping white for the hottest gas only
  made the fire orange through its first half.

## Cost

Bench, 1920x1080, `Timing=1`: 0.16 ms of GPU time a pass for a destroyer's fireball at
RTS distance, 0.36–0.59 ms for a cruiser's or battleship's with its ring, and 0.34 ms
averaged over a battleship's fireball close up (filling a third of the screen). The
frame stayed on its 16.7 ms vsync floor throughout.

## Settings

`Explosions.ini`, section `[Explosions]`; `Reload=1` re-reads it within half a second of
an edit. `Enable`, `Rings`, `Brightness`, `Size`, `MinRadius`, `Growth`, `Cooling`,
`Smoke`, `Turbulence`, `Opacity`, `Flash`, `Glow`, `Detail`, `RingBrightness`,
`RingWidth`, `RingBreakup`, `NoiseSize`, `Timing`, `Log`, and for the bench `Age=`,
which draws every fireball at that fraction of its life (with the game paused, a whole
lifetime can be shot frame by frame). After each reload the log names each fireball on
screen with its age, so a bench shot can be paired with the moment it shows.

## Not covered yet

- The release package (`publish/`, the GUI installer) does not carry the layer.
- The pre-death blasts (`Craft::DramaticDeathSequence`'s `NodeParticleEffect` sprites
  on the hull) and the hull's fragments stay stock.
- Mine and latinum explosions are `FireballExplosion`s too, and get the same fire; the
  self-destruct (`xselfd`) shockwave gets the same ring in its class's white.
- The ring lies in the dead craft's own plane; edge-on it is a line.
- No light of its own: the light on the hulls around it is still `Lighting.asi`'s.
