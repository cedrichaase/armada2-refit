# render3d against the plugins it replaces

What the 3D view costs with render3d, against the same view drawn by the engine with
the eleven old plugins (an older checkout, "Method" below) that render3d's `install.sh` takes out (`Lighting.asi`, `Sky.asi`,
`Nebulae.asi`, `Surfaces.asi`, `Explosions.asi`, `Shields.asi`, `Repair.asi`,
`Venting.asi`, `Specials.asi`, `Fog.asi`, `MSAA.asi`), measured the same way on the
bench. Every other layer (the HUD, the menus, QOL) is installed in both. The instrument is `Timing=1` (README.md, "Timing").

## Method

    render3d/perf/compare.sh <a copy of a stock-plugin checkout> [out dir]

- **old**: main at 342186f, installed by its own `./install` (every plugin above), with
  this checkout's `Render3D.asi` on top in measure-only mode (`Enable=0`, `Timing=1`,
  `perf/probe-old`): nothing of render3d draws; both calls of
  `Armada_RenderAllOurStuff` go through a body that marks the CPU clock and a GPU
  timestamp before and after the engine's own, with every plugin hooked into it.
- **new**: this checkout's `./install`, with `Timing=1` (`perf/probe-new`).
- Each scene of `testbench/scene` (`showcase`: five ships firing at two Borg cubes
  before a planet; `match`: a battle staged in an Instant Action match; `nebula`: a
  Galaxy class at the edge of the Mutara nebula), at 1920x1080, one game at a time,
  80 s after the scene is built, a report every 5 s; the first report is dropped as
  warm-up and the rest averaged (`perf/summary.py`).
- Figures are per main view (the HUD and its inset are off in these scenes). "CPU" is
  the game thread's time in the view, which includes DXVK recording our commands;
  "GPU" is the time between the view's first and last timestamp on the GPU.
- The bench presents with vsync, so both run at 60 fps and the frame period says
  nothing; compare the busy times. The GPU is an RX 5700 shared with the desktop; the
  bench's own compositor is headless.

## Results (2026-10-15)

| scene | CPU, old | CPU, new | worst view CPU, old → new | GPU, old | GPU, new |
|---|---|---|---|---|---|
| showcase | 1.77 ms | **1.27 ms** | 10.2 → 7.8 ms | 0.91 ms | 1.36 ms |
| match | 0.63 ms | 0.66 ms | 1.6 → 1.6 ms | 0.23 ms | 0.62 ms |
| nebula | 1.21 ms | **1.02 ms** | 2.3 → 5.3 ms | 3.69 ms | 4.39 ms |

render3d takes less of the game thread than the engine with its plugins in a battle and
in a nebula, and the same in the match's quieter view. It takes 0.4-0.7 ms more of the
GPU: 0.36-0.41 ms of it is the frame's post passes (resolve of the 4x half-float target,
the bloom chain, the tone map), which the old chain did not have; in the nebula the
volume pass is 3.7 ms in both. Both stay under a tenth of a 60 fps frame.

## What the first measurement found

The first run of the same comparison (logs kept in `$A2_DATA/perf/20261015-render3d/`)
had render3d at 4.74 / 1.48 / 2.03 ms of CPU and a worst view of 24-28 ms in every
scene. Two causes, both fixed:

- **The state saved around each draw.** `gfx::Guard` captured and applied a whole
  `SBT_ALL` state block around every draw of every module, so the engine's state cache
  never sees our state: 0.77 ms capturing and 2.0 ms applying per view in the battle,
  three quarters of render3d's CPU time. It now uses a block recorded with
  Begin/EndStateBlock that holds only what our draws change (`gfx::recorded_block`):
  0.13-0.23 ms. The transforms, lights and materials were what made the whole block
  dear; the shader constants are kept (leaving them out lost most of the starfield).
- **Ini files read under Wine.** `GetPrivateProfileString` opens and parses the file on
  every call, and the sky read its 20-odd keys every 60 views: a hitch of ~20 ms once
  a second. `rt::Ini` now keeps each file parsed and reads it again only when its write
  time or size changes.

Where the rest goes in the battle (`Timing=1`'s zones, CPU per view): the hulls' draws
0.08 ms, their posing 0.07 ms and shadow map 0.03 ms; the sky's 770 star quads,
gathered on the CPU each frame, 0.2 ms; the effects 0.1-0.2 ms.

## A/B: any two checkouts or commits

`compare.sh` measures render3d against the engine with the old plugins. To measure a
change to render3d (or ui, or anything `./install` puts in) against what came before
it, use `ab.sh`, which installs each side whole and times both the same way:

    render3d/perf/ab.sh main HEAD                      # two revisions
    RUNS=3 SCENES=showcase render3d/perf/ab.sh ../other-worktree .   # two directories, three passes

- A side is a checkout directory or a git revision. A revision is exported with
  `git archive` into `~/.cache/armada2-refit/perf-trees/<sha>/` and built there, kept
  for the next run; nothing is built in the user's checkout or a worktree that is not
  named.
- Each side gets its own `./install`, the scene builder and `probe-new` (`Timing=1`).
  `RES` (default 1920x1080), `SETTLE` (seconds after the scene is ready, default 80)
  and `SCENES` (default `showcase match nebula`) as in `compare.sh`; logs and shots go
  to `$A2_DATA/perf/ab-<time>/`, with `sides.tsv` naming what `a` and `b` were.
- `RUNS=N` repeats each pair, alternating the order (a b, then b a), so a machine that
  warms up or drifts does not land on one side. `summary.py` gives a column per side,
  the mean of the runs with their spread (min-max) for the frame, CPU and GPU times,
  and the worst of the runs for the maxima. A difference inside the spread is noise.
- The bench presents with vsync, so compare busy times (CPU and GPU), not the frame
  period, as above.
- **The noise floor of one run** (2026-10-22, `ab.sh main HEAD` on the same commit,
  `showcase`, 1280x720, `SETTLE=25`): 3D view CPU 1.31 against 1.40 ms, GPU 0.84
  against 0.83 ms. A CPU difference under ~0.1 ms in one run is noise; use `RUNS=3`
  for anything smaller.
