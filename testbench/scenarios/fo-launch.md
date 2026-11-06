# Fleet Operations: Roots reaches its main menu with nothing of ours installed

Resolutions: 1280x960
Timeout: 5 min

Roots as it ships, in a clone of the user's install (`a2test --game fo-roots`, or
`A2_PROFILE=fo-roots`): `Data/ArmadaL.exe` launched directly, not through `FleetOps.exe`,
with DxWrapper and DXVK as its Direct3D chain and no layer of ours. Nothing is installed
into the clone (`games/fo-roots.toml` lists no layers), and the bench's `Settings.xml`
adapter has set the display mode. The front end is Roots' own at 1200x900, so the output
must be at least that: at 1280x720 the engine dereferences a null display mode and
madExcept's dialog stands over a black screen (`games/README.md`, "Fleet Operations:
Roots"). Roots writes no log of its own, so the waits read the screen.

1. Launch the game.
2. Wait for the main menu.
3. Take a screenshot called "main menu".
4. Check that the Fleet Operations: Roots title stands over a space station backdrop, with a column of buttons (Single Player, Instant Action, Multiplayer, Map Editor, Options, Credits, Exit) in the middle and a version number below, and no error dialog.
5. Quit the game.
6. Expect the game to have exited.
7. Expect no crash.
