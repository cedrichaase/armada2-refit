# Passthrough with every render3d module off: a Borg mission drawn by Display8

Resolutions: 1280x720
Mod: refit
Clock: bench
Setup: display/bench-mode.sh stub Passthrough=1 render3d/bench-ini.sh Modules:effects=0 Modules:ordnance=0 Modules:specials=0 Modules:sky=0 Modules:overlays=0 Modules:particles=0 Modules:hulls=0 Modules:world=0
Timeout: 15 min

Run with `--install .`. As `display-passthrough`, but with all eight of render3d's
modules at 0 (effects, ordnance, specials, sky, overlays, particles, hulls, world), as a
player who wants the stock look on the stub has it: the engine draws the whole scene and
Display8 draws all of it. The Borg campaign's *Strange Bedfellows* (`a2_borg07`),
entered through the menus as a player does, opens over the player's Borg yard and
stations; this map has lights, which the engine leaves on (`LIGHTING`) for the Borg
bump passes, and until display 0.15.2 the passthrough wrote those lights over the bump
shader's constants, so every Borg hull vanished but its glow (`display/README.md`,
"Passthrough"). The showcase on `a2_borg01` has no such lights and never showed it.

The same mission through the forward chain (`display-passthrough-all-forward`) is the
reference. The stars are dealt anew at every launch, so compare the stations, not the
whole frame (at 1280x720, the yard top right and the station bottom right):

    ./a2test run display-passthrough-all-forward display-passthrough-all --install .
    magick compare -metric RMSE <(magick FWD -crop 260x260+800+0 +repage png:-) \
        <(magick STUB -crop 260x260+800+0 +repage png:-) null:

Measured 2026-10-30: 8 to 9/255 over the yard (24/255 with the stations gone).

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait 5 seconds.
5. Click "Borg Campaign".
6. Wait for "Strange Bedfellows".
7. Click "Strange Bedfellows".
8. Wait 1 second.
9. Click "O.K.".
10. Wait up to 300 seconds for the briefing.
11. Wait 3 seconds.
12. Click "OK".
13. Wait 2 seconds.
14. Click at 55,555.
15. Wait 5 seconds.
16. Take a screenshot called "borg07".
17. Expect "Render3D.log" contains "module hulls: off for missions".
18. Expect "Display8.log" contains "the Borg bump shader drawn with LIGHTING on".
19. Expect "Display8.log" not contain "PASSTHROUGH an engine draw while a plugin records".
20. Expect "Display8.log" not contain "vulkan 0x".
21. Expect no crash.
