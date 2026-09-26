# Changelog — testbench

`./a2test`: headless end-to-end runs of the game at any resolution, with plain-text
scenarios, screenshots and reports. It installs nothing into the game: every case runs
on a reflink clone. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The why is in [`README.md`](README.md).

## 1.5.0 — 2026-09-26

### Added
- `HUD.log` is collected with each case's logs.

### Changed
- The remastered prepare step is `hud/install.sh` (`HUD.asi`), the same for every
  case, in place of `ui-widescreen.py`, `cursor-aspect.py` and `ui-font-condense.py`
  run with `--res` per case.

Installs nothing into the game. Not yet signed off.

## 1.4.0 — 2026-09-26

### Added
- `… within N%` on a stretch check: that step's tolerance, in place of `--tolerance`.

### Changed
- `hud.md`: the three font checks pass within ±10% (measured 0.91–0.94 of stock, judged
  fine by the user in game); the HUD regions stay at ±5%.

Installs nothing into the game. Not yet signed off.

## 1.3.0 — 2026-09-26

### Added
- `Stock shell: embed` (scenario header): stock cases keep `Menus.asi` with `Embed=1`
  and nothing else, so the stock menus take injected clicks (without it the Borg
  campaign click never registered, at 1600x1200 or 800x600). `hud.md` and
  `stock-control.md` use it.

### Changed
- `hud.md` measures against stock at **800x600**, the resolution the game was designed
  for; regions are in 800x600 pixels.
- A comparison with no baseline (the reference case failed or never ran) is SKIP with
  the reason, not inconclusive. Judged comparisons are skipped the same way rather than
  judged blind.

### Fixed
- Silent runs: each session gets its own null sink, made Wine's default output
  (`DefaultOutput` under a pre-registered GUID in the clone's `user.reg`), because the
  game crashes at start-up with no audio device. A `pactl subscribe` watchdog moves,
  mutes and stops any session stream that lands elsewhere; streams not yet linked, and
  streams on any `a2test-` sink, are not leaks.
- Prepare runs `--revert` before `ui-widescreen.py` and `cursor-aspect.py`: at 4:3 both
  do nothing, so a 4:3 case measured the install's 21:9 layout (HUD 0.56x).
- Stock clicks are scaled to the screen: the 800x600 shell mode fills the output.
- OCR reads frames up to 700 px tall at 4x; 2x missed 8 px shell text at 800x600.
- A shot belongs to a case by path, not string prefix (`1600x1200-stock` matched
  `1600x1200`).
- A case cut short by Ctrl-C/SIGTERM is an error, not a PASS.

Installs nothing into the game. Measured run 20260926-030021: stock 800x600 PASS;
remastered HUD within 2% of stock at 4:3, 16:10, 16:9 and 21:9, font 0.91-0.94x.
Not yet signed off.

## 1.2.1 — 2026-09-26

### Changed
- Judged and agent steps run on Sonnet by default (`--model sonnet`), not the `claude`
  CLI's default (Opus here). `A2TEST_MODEL` still overrides it; empty restores the CLI's.

Installs nothing into the game. Not yet signed off.

## 1.2.0 — 2026-09-26

### Added
- `a2test run --jobs N` (`-j`, `A2TEST_JOBS`): up to N cases at once, default 3. A
  scenario's reference case runs first; its other cases are queued once it is done.
  Ctrl-C / SIGTERM stops every running game and waits for its teardown.

### Fixed
- Silent runs: `winepipewire.drv` disabled alongside pulse and alsa. (Superseded in
  1.3.0: with no driver at all the game crashes at start-up.)
- Session ids gain a per-process counter (timestamp and pid collide across threads);
  VNC ports are claimed under a lock; `a2input` is built once before any case starts.

Installs nothing into the game. Not yet signed off.

## 1.1.0 — 2026-09-26

### Added
- `Reference: <aspect> <mod>` and `compared with stock 4:3`: a reference can name its
  mod state. A reference in the other state runs first as its own case
  (`<res>-stock`); the scenario's own 4:3 case is then measured, not skipped.

### Changed
- `hud.md` measures the HUD and font against **stock** 4:3, the shape they were drawn
  for, instead of remastered 4:3. The judge brief no longer excuses a condensed font.

Installs nothing into the game. Not yet run.

## 1.0.0 — 2026-09-25

### Added
- `a2test run | check | list | session | drive`: clone, per-resolution prepare,
  headless sway + Xwayland, umu launch, `a2input` virtual pointer/keyboard, `grim`
  capture, teardown; `index.html` / `report.md` / `results.json` per run.
- The scenario grammar (`scenarios/README.md`): deterministic steps, measured checks
  (OCR, stretch against the 4:3 reference, black bars, flat areas, logs, crash),
  judged steps and agent steps through `claude -p`.
- Scenarios: `main-menu`, `campaign-screen`, `borg-mission-1` (and its prose twin),
  `admirals-log` (menus 2.0.1), `escape-menu` (menus 2.0.2, 2.1.0), `hud` (HUD and
  font, at the Borg briefing), `stock-control`; the `_enter-borg-mission` fragment
  and `Include`.
- `ui.json`: screen signatures and click targets in 800x600 design space.

Installs nothing into the game. Validated by a bench run at 4:3, 16:10, 16:9 and 21:9;
not yet signed off.
