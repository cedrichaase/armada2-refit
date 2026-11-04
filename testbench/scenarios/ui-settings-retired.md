# The hub cleans the retired keys out of the overlay, once (ui 2.2.0, phase 11 R-C3)

Resolutions: 1280x720
Mod: refit
Overlay: Display8.Display8:Backend=d3d9,vulkan Display8.Display8:Mode=forward Display8.Display8:Real=d3d8_real.dll
Timeout: 10 min

`ui/README.md`, "Installing" (the overlay's retired keys), and `ui/src/settings.rs`
(`RETIRED`). A `RefitPlayer.ini` from before display 1.0.0 may hold `Backend=d3d9,...`,
`Mode=` and `Real=` under `[Display8.Display8]`. Display8 skips them on its read side
(`display-retired-keys-overlay`); the UI's hub, the overlay's only writer, removes them at
its first frame: the token `d3d9` goes out of the list (`vulkan` stays), `Mode` and `Real`
go entirely, each logged, and the plugins are told. Run with `--install .`.

    ./a2test run ui-settings-retired --install . -j 1

1. Launch the game.
2. Wait up to 120 seconds for "UI.log" to say "retired".
3. Expect "UI.log" contains "settings: RefitPlayer.ini [Display8.Display8] Backend=d3d9,vulkan retired; now vulkan".
4. Expect "UI.log" contains "settings: RefitPlayer.ini [Display8.Display8] Mode=forward retired; removed".
5. Expect "UI.log" contains "settings: RefitPlayer.ini [Display8.Display8] Real=d3d8_real.dll retired; removed".
6. Expect "RefitPlayer.ini" contains "Backend=vulkan".
7. Expect "RefitPlayer.ini" not contains "d3d9".
8. Expect "RefitPlayer.ini" not contains "Mode=".
9. Expect "RefitPlayer.ini" not contains "Real=".
10. Wait up to 300 seconds for the main menu.
11. Quit the game.
12. Expect the game to have exited.
13. Expect no crash.

A second launch finds nothing to clean: its `UI.log` has no `retired` line (the file is
already clean, which steps 6 to 9 show).
