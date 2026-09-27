# Changelog — cutscenes

`binkproxy/`, a replacement `binkw32.dll`, and the replacement movies in `movies/`.
Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first.
Details are in [`binkproxy/README.md`](binkproxy/README.md).

## 2.0.0 — 2026-09-27

### Changed
- A movie is its recipe, `movies/<Name>/movie.conf`, and a work directory,
  `$A2_DATA/movies/<Name>/` (`ai/`, `src/`, `out/`); `ai-movie.sh`, `build-movie.sh`
  and `install.sh` read and write there. MAJOR: the layout changed.

### Removed
- `A2_MOVIES`, which pointed at recipe and data together.

Installs nothing different. Checked on the bench: the intro installed from `$A2_DATA`
is byte-identical to the one installed today.

## 1.0.1 — 2026-09-27

### Changed
- `install.sh`, `ai-movie.sh` and `build-movie.sh` find the game through `a2env.sh`
  (`A2_GAME`; `A2_GAME_DIR` still read) instead of a hard-coded path.

Installs nothing different.

## 1.0.0 — 2026-09-25

Baseline: the first versioned release.

- `binkw32.dll` proxy: plays `animations/<Name>.mp4` (AV1) plus `<Name>.wav` in place of
  `<Name>.bik`, and forwards everything else to `binkw32_orig.dll`. It raises
  `PlayIntroMovie`'s 640x480 mode to the desktop, so all four launch reels fill the
  screen.
- `movies/Intro`: Bria 2x plus `minterpolate` to 30 fps. Confirmed in game.
- `movies/<Name>/` is laid out like a texture target. `ai-movie.sh` is the only script
  that spends money; `build-movie.sh` rebuilds offline.

## Before versioning

### 2026-09-25
- Moved from `tools/binkproxy/` and `movies/` to `cutscenes/` (`c3ae017`).

### 2026-09-24
- The `binkproxy` DLL (`a3d71ba`) and the `movies/<Name>/` layout (`76e9e23`). The
  upscaled 30 fps intro was confirmed in game (`1c178f9`).
