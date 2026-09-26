# Changelog — msaa

`MSAA.asi`: multisample anti-aliasing via a hook on the engine's `CreateDevice` path.
Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first.
Details are in [`README.md`](README.md).

## 1.0.1 — 2026-09-27

### Changed
- `install.sh` find the game through `a2env.sh` / `a2env.py` (`A2_GAME`, then
  `~/.config/armada2-remastered.conf`, then Heroic's default) instead of a hard-coded
  path.

Installs nothing different; nothing to see in game.

## 1.0.0 — 2026-09-25

Baseline: the first versioned release.

- The present parameters are edited just before `IDirect3D8::CreateDevice`: the highest
  sample count the device accepts, for colour and D16 depth (8x by default),
  `SwapEffect` DISCARD, and the lockable back buffer cleared. The call sites are
  byte-checked, all or nothing. Memory-only. `install.sh --remove` uninstalls it
  completely.
- Confirmed in game at 8x on 2026-09-23. The minimap is the one thing that could break.

## Before versioning

### 2026-09-25
- Moved from `tools/msaa/` to `msaa/` (`7daf6d8`).

### 2026-09-23
- `MSAA.asi` added (`586dd9d`). Confirmed in game, with no DXVK errors (`1aa0208`,
  `822c670`).
