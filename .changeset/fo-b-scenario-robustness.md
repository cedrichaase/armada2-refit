---
testbench: patch
games: [fo-roots]
---

### Fixed
- Fleet Operations: Roots' side picking works on both chains: a `shell` space for `Click at` / `Move the mouse to` (the Instant Action setup screen's coordinates, moved by where LAUNCH is found), so the one fragment `_fo-sides` serves Display8 and the DxWrapper chain; `_fo-sides-chain` is gone.
- `Quit the game` on another game's shell moves to a button and pauses before each click (Roots loses a click after a pointer jump) and, when five tries have not ended the game, stops the prefix and passes (the checks before it still decide; Armada II's quit still fails).
