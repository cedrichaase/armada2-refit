# Claims: Display8 alone, with the UI, and with render3d's sky at 0, on the stub

Resolutions: 1280x720
Mod: refit
Players: alone, ui, render3d
Launch: -nointro a2_borg01
Clock: bench
Setup: display/bench-mode.sh stub Passthrough=1 display/bench-claims.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 15 min

Run with `--install .`. The plugins register with Display8 through the contract
(`display8v1_register`, contract 1.1; `gpu/src/display.rs`, "Claims";
`display/README.md`, "Claims") and Display8 asks each at every `BeginScene` what it
draws now; what nobody claims is the engine's, drawn by passthrough. Three games at
once, each on the stub (`Backend=auto` as shipped: Vulkan on the bench's GPU, with the
validation layer), the showcase scene paused on one tick, `display/bench-claims.sh`
leaving plugins out of the clone by the player's name:

In **alone** nothing claims: no `Render3D.asi`, and the UI with every part at 0
(`[HUD] Enable=0`, `[Loading] Enable=0`, `[UI] Cursor=0`, `[Menus] Credits=0`), so it
registers and claims none; every draw is the engine's, the whole frame stock through
passthrough. In **ui**, Display8 and the UI: it claims its HUD, the selection (its
brackets, the bubbles off), the loading screen and the Credits, and the 3D view is the
engine's. In **render3d**, Display8 and render3d with `[Modules] sky=0` (and the UI's
parts at 0): it claims every module but the sky, which it leaves to the engine (drawn
by passthrough), and the HUD is the engine's.

The UI stays in the clone in **alone** and **render3d** because the game does not
start on the stub without it (a stack overflow before the first `CreateDevice`, seen
2026-10-30 on round-b too): its `[Menus]` part raises the shell's 800x600x16 display
mode to the desktop's, the one mode the stub offers. Display8 with no UI at all on the
stub needs that raise of its own first (`display/README.md`, "Claims").

1. Launch the game.
2. Wait up to 600 seconds for "Scene.log" to say "paused at tick".
3. Wait 5 seconds.
4. Take a screenshot called "claims".
5. Check that the screenshot shows a space battle map of Star Trek: Armada II drawn by the game (stars, ships or stations, or the game's own interface), not a black or garbled screen.
6. Expect "Display8.log" contains "passthrough: the engine's first draw".
7. Alone: Expect "Display8.log" contains "claims: UI.asi claims none".
8. Alone: Expect "Display8.log" not contains "Render3D.asi registers".
9. Ui: Expect "Display8.log" contains "claims: UI.asi registers (id 1)".
10. Ui: Expect "Display8.log" contains "claims: UI.asi claims selection hud loading credits".
11. Ui: Expect "UI.log" contains "gpu: registered with Display8".
12. Ui: Expect "Display8.log" not contains "Render3D.asi registers".
13. Render3d: Expect "Display8.log" contains "claims: Render3D.asi registers (id".
14. Render3d: Expect "Display8.log" contains "claims: Render3D.asi claims hulls world particles effects ordnance specials fog selection".
15. Render3d: Expect "Display8.log" not contains "claims: Render3D.asi claims sky".
16. Render3d: Expect "Render3D.log" contains "module sky: off".
17. Render3d: Expect "Display8.log" contains "claims: UI.asi claims none".
18. Expect "Display8.log" not contains "PASSTHROUGH an engine draw while a plugin records".
19. Expect "Display8.log" not contains "PANIC".
20. Expect no crash.
