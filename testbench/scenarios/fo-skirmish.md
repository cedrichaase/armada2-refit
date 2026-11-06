# Fleet Operations: Roots loads a skirmish map and runs it, with nothing of ours installed

Resolutions: 1280x960
Timeout: 8 min

An Instant Action game, Federation against the Klingon AI (`_fo-sides-chain`: the sides are picked, since
the race is dealt at random and the HUD's "6000" is Federation's), on the map Roots proposes, from the
main menu to the map with its HUD up (`a2test --game fo-roots`; `fo-launch` has the setup).
Nothing logs a tick yet: Roots writes no log, and `FleetOpsHook.dll` is the only code
that could (FO-2 measures it), so "the map is loaded" is the HUD's resource counters on
the screen, and "it runs" is the game staying alive a few seconds later with a picture
that moved.

1. Launch the game.
2. Wait for the main menu.
3. Include "_fo-sides-chain".
4. Click "LAUNCH".
5. Wait up to 180 seconds for "6000".
6. Wait 5 seconds.
7. Take a screenshot called "skirmish".
8. Check that a space map is shown from above with a ship or station of the player's in the middle, resource counters along the top, a minimap at the bottom left and a row of small buttons at the top right.
9. Wait 5 seconds.
10. Take a screenshot called "skirmish later".
11. Quit the game.
12. Expect the game to have exited.
13. Expect no crash.
