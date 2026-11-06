# A later CreateDevice on another window: the swap chain follows it

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: display/bench-mode.sh stub Passthrough=1 BenchWindow=300 render3d/bench-ini.sh Modules:hulls=0 showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 15 min

Run with `--install .`. Fleet Operations: Roots makes a second device, on another window, for
its game (its shell has a windowed one of 1200x900), and a skirmish was a white screen while
the engine kept drawing into the first. Display8's bench-only `BenchWindow` hook hands a new
popup window over the desktop to `ensure_device` at the 300th `Present`, as a later
`CreateDevice` does, and the first again at the 600th: the swap chain moves each time
(`display/README.md`, "A second window"), and the paused showcase must show on whichever
window is current.

1. Launch the game.
2. Wait up to 600 seconds for "Scene.log" to say "paused at tick".
3. Wait up to 120 seconds for "Display8.log" to say "the swap chain follows it".
4. Wait 3 seconds.
5. Take a screenshot called "second window".
6. Wait 15 seconds.
7. Take a screenshot called "first window again".
8. Expect "Display8.log" not contain "could not follow".
9. Expect "Display8.log" not contain "vulkan 0x".
10. Expect no crash.
