# Changelog — gameplay

Map scroll speed (`scrollspeed.py`, which writes `ARMADA.PRF` and `RTS_CFG.h`) and the
cutscene draw-distance notes. `a2mod` does not switch this folder. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The derivation is
in [`README.md`](README.md).

## 1.0.0 — 2026-09-25

Baseline: the first versioned release.

- Scroll tuning applied: a low ramp floor (`INITIAL_SCROLL_SPEED` 1.0) with a high
  ceiling (`MAX_SCROLL_SPEED` 8.0), `SCROLL_COEFFICIENT` 300000,
  `FASTSCROLL_COEFFICIENT` 0.015, `SCROLL_BORDER_WIDTH` 20, and profile mouse 5 /
  keyboard 10. `scrollspeed.py --revert` restores both files.
- Cutscene draw distance: understood and documented (all 52 call sites), deliberately
  not changed.

## Before versioning

### 2026-09-25
- Moved to `gameplay/` (`7daf6d8`). The draw-distance notes were merged in (`e648a3a`).

### 2026-09-23
- Cutscene draw distance documented (`e46940f`).

### 2026-09-22
- Profile scroll speeds and the edge band raised (`cbe455d`). `SCROLL_COEFFICIENT`
  raised, and the ramp flattened (`f150737`).
- The flat ramp removed acceleration, so it was restored with a higher ceiling.
  `scrollspeed.py` added (`bffde2c`).
