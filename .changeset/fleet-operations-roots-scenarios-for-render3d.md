---
layers: [testbench]
bump: minor
---

### Added
- Fleet Operations: Roots scenarios for render3d (FO-5 step 8): fo-render3d-seam, -rings, -debris, -shields, -grid and -stlogo, with the fragments _fo-render3d-skirmish (sides picked, wait on the hull claim in the log) and _fo-render3d-battle. Run with a2test --game fo-roots --install .
- fo-skirmish and fo-display8 pick the sides (Federation against Klingon: fragments `_fo-sides`, in the `shell` space, which serves both Display8 and the DxWrapper chain) and no longer wait on a race-dependent "6000" (fo-display8 waits on Display8's log).
