# Changelog — cutscenes

`binkproxy/`, a replacement `binkw32.dll`, and the replacement movies in `movies/`.
Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first.
Details are in [`binkproxy/README.md`](binkproxy/README.md).

## 2.1.1 — 2026-10-23

### Fixed
- `binkproxy/build.sh` takes a lock (`build/.build.lock`), so two builds of one tree at
  once (`a2test -j` installing one checkout twice) no longer write the same files in
  `build/`.

Installed, not yet seen in game.

## 2.1.0 — 2026-10-23

### Added
- `binkw32.dll`: with `UI.asi` loaded, each full-screen frame (the launch reels and the
  intro, replacement or real Bink) is handed to the UI's `ui_present_movie` to draw,
  not copied into the locked back buffer; without the UI, or when it declines, the
  copy as before. `UseUI=` in `BinkProxy.ini` (default 1) (binkproxy/README.md).

Confirmed in game 2026-10-23.

## 2.0.3 — 2026-10-21

### Changed
- `binkw32.dll` carries a version resource (`VERSIONINFO`): product, description, the
  layer's version as the file version and the project's as the product version, which
  Windows shows in the file's Properties. Built by `platform/versioninfo/build-res.sh` (915cad8).

Confirmed in game 2026-10-22.

## 2.0.2 — 2026-10-12

### Fixed
- `binkw32.dll`: a replacement movie larger than the buffer it is copied into outside
  the fit path (`FitIntro=0`, or the intro left in a narrower mode) is scaled to fit
  the room right of and below (x,y); the 1:1 copy is also clipped to the pitch's width.
  It used to write past the locked back buffer.
- `binkw32.dll`: a `binkw32_orig.dll` missing an export is unloaded again, so later
  calls fail cleanly instead of calling through null pointers.
- `binkw32.dll`: a movie path whose full name does not fit 300 characters is passed to
  the real DLL; it used to read an unwritten buffer.
- `binkw32.dll`: a `.wav`'s block align and byte rate are derived from its channels and
  rate, not trusted from the file (a 0 divided in the log; odd values reached
  `waveOutOpen`).

Installed, not yet seen in game.

## 2.0.1 — 2026-09-27

### Fixed
- `binkproxy/install.sh` refuses only while the game runs from the directory it installs
  into (working directory or `STEAM_COMPAT_INSTALL_PATH`, as `a2mod`'s `game_running()`).
  Any running `Armada2.exe` used to count, so the test bench's `--install` into a clone
  failed whenever the real game was open.

Installs nothing different into the game.

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
