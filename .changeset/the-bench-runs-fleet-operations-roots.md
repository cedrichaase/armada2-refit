---
layers: [testbench]
bump: minor
games: [fo-roots]
---

### Added
- The bench runs Fleet Operations: Roots as it ships (`a2test --game fo-roots`, or A2_PROFILE): a Settings.xml adapter (display mode, fullscreen, no intro video) in place of ARMADA.PRF, Data/ArmadaL.exe launched directly, the plugin and log files looked for beside the executable in Data/ (a descriptor's `[logs]` names the crash file and log folders), a clone with nothing installed for a game with no layers, a session that remembers its game so `drive` and `session stop` need no --game, a refusal of an output smaller than the shell's mode, and the scenarios fo-launch and fo-skirmish
