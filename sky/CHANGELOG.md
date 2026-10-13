# Changelog — sky

`Sky.asi`: the map's sky drawn from a recipe, in a shader, in place of the six painted
cube faces, and the recipes in `skies/`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.3.0 — 2026-10-13

### Added
- Recipe keys `GradeGain=` / `GradeCurve=`: a per-channel grade (`gain * in^curve`) of
  the gas, and `sky/grade.py`, which fits them to the stock set from a bake's faces.
- Recipe key `Fine=` (amount, size, gain; default 0.1, 0.3°, 0.55): detail finer than
  the cube's texels, three octaves of a tiling noise volume multiplied in at draw time.
- `Sky.ini` key `Dump=` (0): writes each bake's faces to `Sky\<name>.dump0..5.tga`;
  `install.sh --remove` removes them. The log names the field of view and the screen
  pixels per cube texel.

### Changed
- Every recipe but `mbgaqu` graded to its stock set: luminance at 0.89–0.92 of stock
  (was 0.79–1.18), channel balance matched; `mbgrg` balance only, at 0.57 of stock.
- `mbgkling`: `Deep` 14,3,0 → 48,9,3 and `Glow` 110,70,22 → 110,86,20 (dark red gaps
  and yellower highlights, as stock).
- The bake takes four samples a texel (2x2, averaged), against aliasing in the
  grain's finest octaves. Re-bakes 60 → 250 ms.

### Fixed
- The dither drew a faint square grid over dark sky: interleaved gradient noise on the
  screen pixel in place of the lattice hash at continuous directions.

Installed, not yet seen in game.

## 1.2.3 — 2026-10-13

### Changed
- `mbgrg` (the green sky of `a2_kling01` and four other maps) toned down: `Softness`
  1.4 → 2.6, `Coverage` 3.2 → 2.1, `Gamma` 1.3, `Grain=0.6,4,0.5`, `Haze=0.5,1.6`,
  `Brightness` 0.307 → 0.25. Soft cloud in place of hard lime clumps; on the bench the
  brightest gas fell from 202 to 98 (green, of 255) and the mean by a third.

Installed, not yet seen in game.

## 1.2.2 — 2026-10-12

### Fixed
- Rebuilt with platform 3.4.1's `d9_device()`, which tells a new Direct3D device by the
  d3d9 device rather than the d3d8 device's address, so the sky's shaders and cube are
  made again on a device at a re-used address.

Installed, not yet seen in game.

## 1.2.1 — 2026-10-12

### Fixed
- The viewport is put back after the bake: the frame's first sky draw covered the whole
  back buffer when the bake happened in a smaller view (the HUD's action camera).
- A timing query pair is never left half made (`Timing=1`).

Installed, not yet seen in game.

## 1.2.0 — 2026-10-12

### Added
- Recipe keys for the detail the stock faces had and the recipes lacked: `Grain=`
  (amount, scale, gain: clumps and mottling), `Dust=` (amount, scale, coverage: dark
  lanes and specks), `Haze=` (amount, coverage: a soft glow behind the clumps),
  `Stars=` (count, brightness, size, in-gas weight) with `StarA=`/`StarB=` (bright
  coloured stars with halos in the gas). Each off unless a recipe sets it. The bake's
  face constants move from c14 to c19. `d1d15e2`

### Changed
- Every recipe uses them: clumps on black for `mbgred`, `mbgrg`, `mbgdom1`, `mbgpur`,
  `mbggb`; dust and grain for `mbgborg`, `mbgkling`, `mbgdk`, `mbgflu`, `mbgaqu`,
  `mbg02`; stars for `mbgbaku`, `mbgpur`, `mbgkl`. Each `Brightness` re-scaled to keep
  1.1.0's mean luminance.

### Fixed
- `mbgblue` was never calibrated (`Brightness=1.0`, about 3.5 times `mbg02`'s); it is now
  `mbg02`'s recipe, as its header says.

Installed, not yet seen in game.

## 1.1.0 — 2026-10-09

### Added
- Recipes for every sky a stock map uses: `mbgpur`, `mbgred`, `mbgrg`, `mbgdk`, `mbgkl`,
  `mbgflu`, `mbggb`, `mbg02`, `mbgblue`, `mbgborg`, `mbgdom1`, `mbgbaku`. Each
  `Brightness` is calibrated to about 0.92 of the stock set's luminance.
- Recipe key `Band=` (yaw, pitch, half-width, gather): gas gathered about a great circle.

Confirmed in game 2026-10-09.

## 1.0.0 — 2026-10-09

### Added
- `Sky.asi`: the call to `ST3D_Instance::Render` in `Background_Render` (0x590e17) draws
  the sky from `Sky\<name>.ini` when the map's sky has one and a Direct3D 9 device is
  behind d3d8; otherwise the stock cube, as before. Domain-warped value noise, baked
  once per map into a 10-bit managed cube with edge texels on the edges, sampled per
  frame.
- `Sky.ini`: `Enable`, `Face` (cube edge, 1536; 0 = per pixel), `Reload`, `Timing`, `Log`.
- Recipes `mbgaqu` (Aqua: dim teal and blue curtains) and `mbgkling` (Klingon: red,
  orange and gold billows about two cores).
- `install.sh` (`--timing`, `--remove`); `./install` runs it, `a2mod` switches `Sky.asi`,
  `Sky.ini` and `Sky/*.ini` as the `sky` layer.

Confirmed in game 2026-10-09.
