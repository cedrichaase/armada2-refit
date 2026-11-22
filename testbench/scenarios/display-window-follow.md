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
`CreateDevice` does, and the game's window again at the 600th, hiding the popup: the swap
chain moves each time (`display/README.md`, "A second window"), and the paused showcase must
show on whichever window is current.

The bench's sway puts the popup behind the full-screen game window, where nothing drawn on it
shows (the screen stays black: the game's window lost its swap chain) and Xwayland lets
through only a few presents a second; the step that makes it full screen is the desktop's
part, which the swap chain's move does not decide. Hidden again, the popup gives the screen
back to the game's window. Run on Direct3D 12 with
`display/stub-scenario.sh display-window-follow stub Backend=d3d12`.

1. Launch the game.
2. Wait up to 600 seconds for "Scene.log" to say "paused at tick".
3. Wait up to 120 seconds for "Display8.log" to say "the swap chain follows it".
4. Put the window "Display8 bench" full screen.
5. Wait 3 seconds.
6. Take a screenshot called "second window".
7. Expect no black bars.
8. Wait up to 60 seconds for "Display8.log" to say "back on the game's window".
9. Wait 5 seconds.
10. Take a screenshot called "first window again".
11. Expect no black bars.
12. Expect "Display8.log" not contain "could not follow".
13. Expect "Display8.log" not contain "vulkan 0x".
14. Expect no crash.
