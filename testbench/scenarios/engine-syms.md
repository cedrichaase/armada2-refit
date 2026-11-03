# Every plugin on its symbol profile: the build recognised, every symbol resolved

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh stub
Timeout: 20 min

Run with `--install .`. On GOG's 1.1 every plugin that has moved onto its symbol
profile (`crates/a2engine/README.md`) logs the executable's identity, recognises the
build, resolves all of its symbols with no hook blocked, and no hook stands down or
has a symbol read before its hook was ready. One game straight into a mission, so the
hooks that are readied late (the stub's mode requests at the first `Direct3DCreate8`,
`[Drop]`'s ranges at the engine's first scene) have run. Each package of phase 8
appends its plugin's section and touches no other.

## Mission
1. Launch the game.
2. Wait 60 seconds.
3. Wait 20 seconds.
4. Take a screenshot called "mission".
5. Check that the screenshot shows a Star Trek: Armada II mission (its HUD around the map view; with no scene staged the 3D view may be empty), not a black or garbled screen.

## Display8
6. Expect "Display8.log" contains "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321".
7. Expect "Display8.log" contains "engine: build armada2/gog-1.1".
8. Expect "Display8.log" contains "engine: symbols: 15/15 resolved; 0 hooks blocked".
9. Expect "Display8.log" contains "stub: the engine's own mode requests (6 of 6 calls)".
10. Expect "Display8.log" not contains "stands down: ".
11. Expect "Display8.log" not contains "read before its hook".
12. Expect "Display8.log" not contains "engine: TEST:".
13. Expect "Display8.log" not contains "PANIC".

## End
14. Quit the game.
15. Expect no crash.
