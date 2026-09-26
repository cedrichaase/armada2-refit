# postfx — renderer settings and bloom

Two `a2mod` layers in one folder: **renderer** (`renderer-config.sh`, which writes
`dxvk.conf`) and **bloom** (`postfx.py` plus the per-user vkBasalt build in
`vkbasalt/`). Both apply to every frame at once and are independent of the texture
pipeline. Anti-aliasing is covered here too, as Tier 2, because it was chosen
among these routes; the hook itself lives in `msaa/`.

Everything in this section is global — it applies to every texture and every model at
once, costs nothing per-asset, and is reversible by deleting one file. All of it is
independent of the texture pipeline. Verified against **DXVK 3.1.1**, the version Heroic
actually deploys (`~/.config/heroic/tools/dxvk/dxvk-3.1.1/`), by reading the config keys
out of the shipped `d3d9.dll` rather than from documentation — the key list below is what
this binary honours, not what some DXVK version honours.

## This does not have to wait for the d3d8 regression

An earlier revision of this file said the graphics stack should not be touched while the
chain regression above is open. For `dxvk.conf` specifically **that is wrong, and the
reason matters**: both possible chains end in DXVK's `d3d9.dll`.

    proxy -> DXVK d3d8   -> DXVK d3d9 -> Vulkan      (live today)
    proxy -> GOG d3d8to9 -> DXVK d3d9 -> Vulkan      (after the regression fix)

`d3d9.*` keys are read by the d3d9 layer, which is present and is DXVK's in both. So a
`dxvk.conf` change is attributable regardless of which `d3d8.dll` won the last launch,
and does not need the regression closed first. The AA routes below are the ones that do.

## Tier 1 — `dxvk.conf`, free and reversible

`postfx/renderer-config.sh` writes it, in **cumulative stages**, because this project's
method is one change at a time and three keys at once is not attributable.
**All three stages are installed and confirmed in game.** It verifies every key against
the `d3d9.dll` that actually loads — the one in the game directory — before writing,
because a key DXVK does not recognise is silently ignored:

    postfx/renderer-config.sh              # stage 1: anisotropic filtering only
    postfx/renderer-config.sh --stage 2    # + mip LOD bias
    postfx/renderer-config.sh --stage 3    # + seamless cube filtering
    postfx/renderer-config.sh --bias -0.25 # milder stage 2 bias (default -0.5)
    postfx/renderer-config.sh --show       # what is installed now
    postfx/renderer-config.sh --remove     # delete dxvk.conf, full revert

- **`d3d9.samplerAnisotropy = 16`** (stage 1). `dxcfg.ini` asks for `application`, i.e.
  whatever a 2001 renderer requests, which is likely none. In a top-down RTS every hull
  and every planet is drawn at a steep angle to the camera, so all of it is sampled with
  plain trilinear and blurs along the axis of foreshortening. This is the single biggest
  free win and the one to judge on its own.
- **`d3d9.samplerLodBias = -0.5`** (stage 2). Biases mip selection toward the sharper
  level. This matters *because of the texture work*, not independently of it: there is
  now 1024–2048 art under a camera that views the map plane obliquely, and trilinear
  picks a blurrier mip than the art can support. **Only safe with AF already on** —
  without it a negative bias aliases rather than sharpens, which is why it is a separate
  stage and not part of stage 1. `d3d9.clampNegativeLodBias` is the guard if it
  overshoots. **The failure mode is shimmer on movement, not blur when
  still** — and in this game it shows on the billboard layer first, so if the map crawls
  while scrolling, halve the bias with `--bias -0.25` before abandoning the stage.
  Below about -1.0 it aliases faster than AF can clean up.
- **`d3d9.seamlessCubes = True`** (stage 3). Filters across cube-map face edges. Directly
  relevant to the finished skybox class: many maps bind a `.sod` cube model, and a
  face-edge seam gets *more* visible at 2048/face, not less.

`DXVK_HUD=fps,frametimes` is the measuring aid; it is deliberately **not** written into
`dxvk.conf`, so it cannot be left on by accident. Expect the GPU to be near-idle — a 2001
engine against an RX 5700 XT — and all three stages to be free.

## Telling whether a renderer setting did anything

Asked after stage 1 went in and the answer was not obvious by eye. It is two separate
questions and they need separate tools, because "the setting was ignored" and "the
setting worked and is subtle" look identical in game.

**Is it applied?** A `dxvk.conf` key DXVK does not recognise is *silently ignored* — no
error, no warning. DXVK does print its effective configuration at startup, so ask it:

    platform/dxvk-logging.py --diagnose  # logging + HUD + d3d9=n,b, all at once
    # launch the game once
    platform/dxvk-logging.py --check     # report the effective configuration
    platform/dxvk-logging.py --off       # take all of it back out again

`--diagnose` answers three questions in one launch, ordered so each makes the next
meaningful:

| observation | conclusion |
|---|---|
| no DXVK HUD overlay in game | DXVK is not in the chain; **no** `dxvk.conf` key can work |
| HUD shown, but no log file | DXVK's d3d9 layer never loaded |
| log written, keys absent from `--check` | DXVK ran but never found or read `dxvk.conf` |
| keys present in `--check` | it is applied, and the effect really is that subtle |

**Why `d3d9=n,b` is part of the diagnosis.** DXVK's `d3d8.dll` imports `d3d9.dll` by
name — confirmed with `objdump`, not assumed — so every `d3d9.*` key is read by a layer
that only exists if Wine resolves `d3d9` to DXVK's build rather than its own builtin
WineD3D. This prefix has only ever carried `winmm=n,b;d3d8=n,b`, and setting
`WINEDLLOVERRIDES` at all *replaces* whatever Proton would have set, so the d3d9 slot
may never have been native. That is the leading suspect whenever a `d3d9.*` key appears
to do nothing here, and it is the same gap the "Also outstanding" note below has
recorded, untested, all along. `--off` restores the original override string too.

**Heroic must be closed** for `--on`/`--off`: it rewrites `GamesConfig` on exit and
would discard the edit. The script refuses rather than losing the change silently, and
backs the file up regardless. Note Heroic's key is spelled `enviromentOptions`, missing
an `n` — matching its typo is required.

**Did it change the picture?** `platform/ab-shot.sh` grabs frames and diffs them
numerically, so the answer is a number rather than an impression:

    platform/ab-shot.sh grab before
    # change one thing, relaunch, return to the same save without moving the camera
    platform/ab-shot.sh grab after
    platform/ab-shot.sh diff before after 600 400 1200 300     # W H X Y, region only

For a before/after of the project as a whole rather than one setting, `./a2mod stock`
and `./a2mod remastered` flip every visual layer at once in about 4 s, with DXVK kept
in both states, so the two grabs differ only in what this project changed. Quit the game
between them; Heroic can stay open. Bloom is bypassed at launch in stock, and Home
still toggles it.

Aim at a region, not the whole frame: a whole-frame diff of this game is dominated by
ships drifting and sprites animating between the two grabs, which will swamp the effect
and make any setting look like it did something.

## Why anisotropic filtering is structurally quiet in THIS game

Worth stating plainly, because the usual "AF transforms an old game" advice assumes
content this game does not have. AF only acts on surfaces **oblique to the camera**, and
it is a correction to *mip selection* — so it can only touch geometry that is both
mipmapped and foreshortened.

- **The dominant visual layer here is immune by construction.** Map nebulae, resource
  clouds, weapons and explosions are camera-facing **billboards**. A billboard is never
  oblique — that is what makes it a billboard — so AF cannot affect any of it. In a
  frame like the one that prompted this, that is most of what the eye is drawn to.
- **What AF *can* reach is small on screen.** Hull flanks, planet limbs and the skybox
  at grazing angles. A Sovereign draws ~340px wide in a top-down RTS, so its
  near-grazing side surfaces are a few dozen pixels tall. Real, and not dramatic.

So a subtle result at stage 1 is the expected result, not evidence of a broken setting —
which is exactly why `--check` exists to separate the two.

**The corollary is the useful part: stage 2 should be the visible one here.** A mip LOD
bias is not conditional on obliquity — it shifts mip selection for *every* mipmapped
surface, camera-facing billboards included. That is precisely the layer AF cannot touch
and precisely where the upscaled art lives. Expect stage 2 to do more for this game's
appearance than stage 1, which inverts the usual ordering. Stage 1 still goes first:
AF is what keeps the sharper mips stage 2 selects from aliasing on the oblique surfaces.

## Tier 2 — anti-aliasing: route 2 built, `msaa/`

**Route 2 is built, installed and confirmed in game: `MSAA.asi`, 8x.** The first launch
logged 8x on all three device creations (3440x1440, the 640x480 fallback, 3440x1440
again) and DXVK logged no errors.

    msaa/install.sh                 # build + install (Samples=8)
    msaa/install.sh --samples 4     # or 2; 0 patches nothing
    msaa/install.sh --remove        # three files out, stock behaviour back

It redirects the two `call GetWindowHandle` that sit just before
`IDirect3D8::CreateDevice` in `ST3D_DeviceDirectX8::CreateDevice` (`0x6235d0`). At
that point `edi` is the present-parameters struct. The plugin then sets:
- the highest sample count the device accepts for both the colour and D16 depth
  formats
- `SwapEffect=DISCARD`
- a non-lockable back buffer

`Reset` reuses the same struct, so a device loss keeps MSAA. `MSAA.log` records what
each device creation actually got. The renderer was read end to end first: nothing
renders off-screen, and the one back-buffer read-back is the minimap copy
(`CopyOffscreenToTexture`). That copy works only because DXVK's D3D8 CopyRects
resolves a multisampled source through `StretchRect`. So **check the minimap first**
when judging it. Full reasoning in `msaa/README.md`.

What follows is the analysis that chose the route.

**DXVK 3.x ships no MSAA-forcing key.** Read out of the binary that actually loads, not
from documentation: there is no `forceSwapchainMSAA` or equivalent.
`d3d9.forceSampleRateShading` exists but only does anything once MSAA is already on. So
AA cannot come from `dxvk.conf`, and the three routes are now these — note that the
cheapest one died when the chain changed:

1. **~~`dxcfg.ini`'s own `antialiasing=` key~~ — dead.** It is read by the GOG d3d8to9
   translator, and the working chain replaced the translator with DXVK's own d3d8.
   `dxcfg.ini` is inert and will stay inert unless the chain moves to
   `d3d8-chain.py --use gog`, which is a real option (GOG d3d8to9 → DXVK d3d9 → Vulkan)
   and would make both `anisotropic=` and `antialiasing=` live again. Untested, and its
   accepted values are unknown because `dxcfg.exe`'s strings are packed.
2. **An ASI hook on `IDirect3D8::CreateDevice`**, setting `MultiSampleType`. Squarely
   inside the toolchain `menus/` already proves — clang + lld-link + the
   Ultimate ASI Loader that is already carrying two plugins — and `armada2.map` gives
   the call site. **The most likely to work, and now the most attractive**: DXVK
   implements D3D8 multisampling properly on Vulkan, which wined3d's D3D8 path did not
   reliably do, so this became a better bet the moment the chain changed.
3. **Post-process AA via vkBasalt** — see Tier 3. FXAA/SMAA rather than MSAA, so it
   softens edges rather than resolving them, but it costs no code at all.

**Supersampling by rendering above display resolution remains a trap.** It looks free
given the GPU headroom and is not: `ui-widescreen.py`'s canvas arithmetic, the
`popupPaletteXA` correction and `Menus.asi`'s desktop-size read are all tuned to
3440x1440 and would need re-deriving.

## Tier 3 — post-processing: bloom done, the rest untried

**This was impossible until the chain changed, and that is the point.** vkBasalt is a
**Vulkan layer**. Until DXVK went in, Direct3D 8 landed on wined3d/**OpenGL**, so
vkBasalt had nothing to attach to — any attempt would have done nothing, with no error
to explain why. Now that the game renders through Vulkan, the whole post-processing
family is reachable for the first time.

**Done: bloom is installed and confirmed in game** — MagicBloom through vkBasalt,
threshold 6, intensity 0.08, accepted by the user after one step up from 0.05.
**Home toggles it live**, which makes it the one renderer setting in this project with a
same-frame A/B.

    postfx/vkbasalt/build.sh          # build + install the layer (per-user, pinned)
    postfx/postfx.py --on             # write config, prove it compiles, enable in Heroic
    postfx/postfx.py --set --intensity 0.08 --threshold 6    # retune; relaunch to see
    postfx/postfx.py --check          # read vkBasalt's log from the last launch
    postfx/postfx.py --off            # disable for the game
    postfx/vkbasalt/build.sh --remove # uninstall everything

How it is put together, and why each piece is that way:

- **Built from source into `~/.local`, not from the AUR.** `lib32-vkbasalt` wants a
  sudo password and installs system-wide; the build needs only what is already here
  (`g++ -m32`, `lib32-libx11`, `glslang`) plus meson/ninja in a venv and headers fetched
  by commit. Every source is pinned by SHA. vkBasalt 0.3.2.10 (2023, the last tag)
  does not build against current Vulkan-Headers — `vk_layer.h` left that repo — so the
  headers are pinned to SDK 1.3.250.
- **32-bit only**, because a Vulkan layer loads into the process that calls Vulkan, and
  that is the i386 game. This holds because Proton-CachyOS uses *old* WoW64 unless
  `PROTON_USE_WOW64=1` is set — **under new WoW64 the Unix side is 64-bit and this
  layer would silently never load.** If that variable ever appears, build a 64-bit one.
- **The manifest is inert without `ENABLE_VKBASALT=1`**, which `postfx.py` sets in this
  game's Heroic environment only, beside `VKBASALT_CONFIG_FILE` and a log file.
- **Nothing lives in the game directory.** vkBasalt's config parser drops spaces and
  warns against them in shader paths, and the game directory is `Star Trek Armada II`;
  config, wrapper, shaders and log are all under `~/.local/share/a2-vkbasalt/`.
- **Shaders are split across two branches of crosire/reshade-shaders**, which is not
  obvious: the single-file bloom shaders are on `legacy`, and `legacy` ships **no
  `ReShade.fxh`** — the headers every one of them includes are on `master`.
- **`fxcheck`** (`postfx/vkbasalt/fxcheck.cpp`) links the layer's own `libreshade.a` and
  compiles an effect with vkBasalt's exact macros and codegen flags, offline.
  `postfx.py` refuses to enable an effect it cannot compile, and refuses any config key
  the compiled effect does not expose. Both failures are otherwise *silent* in game — an
  absent effect, an ignored key — which is the DXVK lesson applied in advance. It caught
  the missing headers on its first run; the output also passes `spirv-val`.
- **`MagicBloom` is wrapped, not used as shipped** (`A2Bloom.fx`, written by
  `postfx.py`): eye adaptation off, because it rescales bloom by average frame
  brightness and would pulse as the camera pans between empty space and a nebula; lens
  dirt off, because there is no lens. Those are preprocessor switches, which
  `vkBasalt.conf` cannot set.

**The numbers were measured before the first launch**, by approximating the shader —
threshold power, 8-level blur pyramid, Hable tonemap at MagicBloom's fixed 100x, screen
blend — in ImageMagick on a real frame:

| threshold / intensity | latinum | fog of war | HUD | whole frame |
|---|---|---|---|---|
| 2 / 1 (MagicBloom's defaults) | — | — | — | mean +123/255, 99.7% of pixels changed |
| 4 / 0.1 | +27.2 | +6.5 | +6.3 | |
| 6 / 0.1 | +22.5 | +0.7 | +3.8 | |
| 6 / 0.03 | +7.7 | +0.2 | +1.2 | |

The threshold is an *exponent* on colour, so it decides **what** blooms and intensity
decides **how much**. At 6 the glow lands on the additive sprites that already clip and
leaves the grey shroud alone (31:1); at 4 the shroud hazes over (4:1). The HUD is in
the frame vkBasalt sees, so it can bloom too; the high threshold is also what keeps
that small. The shader's own defaults would have made the frame milky.

It operates on the final swapchain image, and that decides what is possible: CAS
sharpening, FXAA/SMAA, LUT/tonemapping and depth-independent bloom all work; anything
needing depth does not.

**Bloom is the one worth doing**, and the reason is specific to this game rather than
general taste. Armada II's visual language is almost entirely additive sprites —
nebulae, weapons, engine glows — and the latinum clouds already clip to flat white where
overlapping billboards composite past 255. That clipping is stock-faithful and cannot be
fixed in the texture, because the per-channel mean *is* the light contributed and
lowering it breaks the match against stock. Bloom converts that blowout from "the
texture ran out of range" into "that is a bright object", which is the correct read and
which no amount of texture work can produce.

Judge it the way the texture repository's `textures/tools/measure-invention.sh` judges a blend, not at 1:1 — and
`platform/ab-shot.sh` will diff two launches numerically.

## Tier 0 — ambient occlusion, which is the wrong tool here

Unchanged by any of the above, and the reasoning is worth keeping because "add AO and
bloom" is the reflex suggestion for any old game and only half of it survives contact
with this one.

The practical objection: depth-buffer access through d3d8 → d3d9 → DXVK is exactly where
ReShade's depth detection is least reliable.

The real objection is about content. **AO darkens contact points and creases, and this
scene has neither.** Ships float in vacuum against a skybox — nothing touches anything,
there is no ground plane, no architecture, no interior corners. The 2001 hull models are
low-poly, so there are barely any geometric creases to occlude either, and what detail
exists is *painted into the diffuse map*, where AO cannot see it. The cost is high and
the return is faint rim-darkening on ship silhouettes.

The same reasoning rules out most lighting-based effects: the game has close to no
lighting model to enhance. That is why the texture work has so much leverage here and
why shader tricks have so little.
