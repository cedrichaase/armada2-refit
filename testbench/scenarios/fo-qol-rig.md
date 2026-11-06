# Fleet Operations: Roots with QOL, RigIgnoresSelf=1: the hook runs beside Roots' own placement; the stations' parts stand down

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d qol
Setup: display/bench-mode.sh stub Backend=vulkan
Overlay: QOL.QOL:RigIgnoresSelf=1
Timeout: 14 min

FO-7 round three (`PLAN-FLEETOPS.md`, "FO-7: QOL on Roots", decisions 111 to 113).
`qol.rig_self` is verified on this build: `ConstructionRig::GetAction` is stock's byte for byte,
in the file and in memory, and FleetOpsHook's `CraftClass::CanPlaceHere` (replaced at its entry
by its own port) passes the object to ignore on to the placement check, as `CraftProcess::Build`'s
arrival check still passes the rig. The key is 1 here by an overlay (the Roots default is 0, see
`fo-qol-rig-off`): the stubs run, QOL.log says the spot is clear with the ship ignored, the ghost
over the ship is green and the click builds. It is green with the key at 0 too, which is why
the feature is off on Roots. `qol.stations` is disabled by the profile (Roots replaces `Select`
and the bar's update), `qol.warp` is patched but the key is 0 (Roots has no warp in its games).

An Instant Action game as Federation (`_fo-sides`); the first Mediterranean class construction
ship is at about 335,640. The bar of Roots is FleetOpsHook's: with the ship selected the fifth
button (599,787) opens the construction menu and its third (459,787) is the Antares Type Shipyard,
whose ghost follows the pointer (green where it can stand, red where it cannot). QOL.log says
`rig: the blueprint's spot is clear|blocked` (with `(the construction ship ignored)` while the key
is 1) when the answer changes. With the key at 0 the same ghost is red (`fo-qol-rig-off`).

1. Launch the game.
2. Wait for the main menu.
3. Expect "QOL.log" contains "build armada2/fo-roots-5.0Z11".
4. Expect "QOL.log" contains "RigIgnoresSelf=1  -> a construction ship does not block its own blueprint, patched".
5. Expect "QOL.log" contains "StationGroups=0  (stations select and group as in stock)".
6. Expect "QOL.log" contains "qol.stations stands down".
7. Expect "QOL.log" contains "WarpDistance=0  (0: moves left alone)".
8. Expect "QOL.log" not contain "patch refused".
9. Include "_fo-sides".
10. Click "LAUNCH".
11. Wait up to 180 seconds for "6000".
12. Wait 5 seconds.
13. Note "select the first Mediterranean class ship, open the build menu, take the Antares Type Shipyard".
14. Move the mouse to 335,640.
15. Wait 1 second.
16. Click at 335,640.
17. Wait 2 seconds.
18. Move the mouse to 599,787.
19. Wait 1 second.
20. Click at 599,787.
21. Wait 2 seconds.
22. Move the mouse to 459,787.
23. Wait 1 second.
24. Click at 459,787.
25. Wait 1 second.
26. Move the mouse to 336,700.
27. Wait 1 second.
28. Move the mouse to 336,640.
29. Wait up to 20 seconds for "QOL.log" to say "the blueprint's spot is clear (the construction ship ignored)".
30. Take a screenshot called "ghost over the ship".
31. Check that the large translucent blueprint of a shipyard is drawn over the Mediterranean class ship at the left, tinted green, and not red.
32. Wait 3 seconds.
33. Click at 336,640.
34. Wait 6 seconds.
35. Take a screenshot called "building".
36. Check that the resource counters at the top of the screen no longer read 6000 (the shipyard was paid for, so the order was sent) and that a station frame is rising beside the ship.
37. Expect "QOL.log" not contain "patch refused".
38. Expect "Display8.log" not contain "VUID".
39. Expect "Display8.log" not contain "device removed".
40. Quit the game.
41. Expect the game to have exited.
42. Expect no crash.
