# Fleet Operations: Roots with the UI's loading screen switched off: the engine's own load bar, and the hook only drawing over it

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Loading.Enable=0
Timeout: 12 min

The A side of the loading screen's A/B (`fo-ui-loading` is the B): `[Loading] Enable=0` gives
the load back to the engine, so Roots' own load bar and logo scene are on screen. The
overlay hook (`loading.overlay`) is still installed over the engine's frame (it draws
nothing of the loading screen); the splash and status hooks are not. Run with
`a2test --game fo-roots` and `--install .`.

1. Launch the game.
2. Wait for the main menu.
3. Include "_fo-sides".
4. Click "LAUNCH".
5. Wait 12 seconds.
6. Take a screenshot called "engine load".
7. Check that this is Fleet Operations' own loading screen (its load bar and logo scene, not a starfield with a players panel and a status line of ours).
8. Wait up to 240 seconds for "UI.log" to say "loading: frame 20".
9. Expect "UI.log" contains "loading: hook at".
10. Expect "UI.log" not contain "a screen of ours".
11. Expect "UI.log" not contain "first frame of ours".
12. Expect "UI.log" not contain "loading: standing down".
13. Wait 25 seconds.
14. Take a screenshot called "match".
15. Check that the match has started: the 3D view of a space map with ships and Fleet Operations' own HUD, and no loading screen left over it.
16. Expect "Display8.log" not contain "VUID".
17. Expect "Display8.log" not contain "device removed".
18. Quit the game.
19. Expect the game to have exited.
20. Expect no crash.
