# Changelog — display

`d3d8.dll`, "Display8": the game's Direct3D 8, ours. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The reasoning is
in [`README.md`](README.md).

## 0.1.0 — 2026-10-23

### Added
- Display8 in forward mode, on the bench only (`display/install` refuses anything but
  an `a2test` clone): a `d3d8.dll` of ours in front of the real chain, renamed
  `d3d8_real.dll`, with an object of ours for every Direct3D 8 interface and every
  method passed through. With `Trace=1` (`Display8.ini`) it counts every call by
  context (`Display8.mark`), method and caller into `Display8.inventory.tsv`, logs each
  distinct answer the engine branches on, every lock, read-back and gamma ramp, and the
  `user32`/`gdi32` calls that read the mode or move the mouse.
- `[Drop] Callers=`: draws whose call chain holds one of these return addresses are not
  passed on (read again while the game runs); `Display8.go` logs the engine's draws of
  the next `[Trace] Frames=` frames with their state and vertices.

Installed, not yet seen in game.
