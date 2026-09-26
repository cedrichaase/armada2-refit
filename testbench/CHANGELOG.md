# Changelog — testbench

`./a2test`: headless end-to-end runs of the game at any resolution, with plain-text
scenarios, screenshots and reports. It installs nothing into the game: every case runs
on a reflink clone. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The why is in [`README.md`](README.md).

## 1.2.0 — 2026-09-26

### Added
- `a2test run --jobs N` (`-j`, `A2TEST_JOBS`): up to N cases at once, default 3. A
  scenario's reference case runs first; its other cases are queued once it is done.
  Ctrl-C / SIGTERM stops every running game and waits for its teardown.

### Fixed
- Silent runs: `winepipewire.drv` (Proton-CachyOS's PipeWire driver, which this
  prefix uses) is disabled too, the clone's `user.reg` gets `Audio=""`, and a watchdog
  mutes and stops the game if a stream of the session's appears at any point in a case.
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
