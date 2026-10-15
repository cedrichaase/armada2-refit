# Changelog — render3d

`Render3D.asi`: the 3D view drawn through render3d, in Rust. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The reasoning is
in [`README.md`](README.md), the plan in [`PLAN.md`](PLAN.md).

## 0.1.0 — 2026-10-15

### Added
- `Render3D.asi`: both calls of `Armada_RenderAllOurStuff` (0x59886e, the main view;
  0x4e60f6, the HUD's action camera) go through render3d. The engine's steps draw into
  a half-float target with 4x MSAA, which is resolved, bloomed by energy and
  tone-mapped into the back buffer at the view's rectangle; the engine's depth is
  cleared there for the HUD. `Render3D.ini`: `Enable`, `Log`, `MSAA`, `HDR`, `Bloom`,
  `BloomThreshold`, `BloomKnee`, `Exposure`, `Shoulder`, `Inset`.
- `install.sh` builds it (rustup, `i686-pc-windows-msvc`) and removes `MSAA.asi`, which
  it supersedes; `./install` runs it in MSAA's place, and `a2mod` switches it as the
  `render3d` layer.

Installed, not yet seen in game.
