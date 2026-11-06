# Fleet Operations: Roots with QOL, RigIgnoresSelf=0: Roots' own ghost already stands on its construction ship

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d qol
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 12 min

The other half of `fo-qol-rig` (FO-7 round three, decision 113): with the key at 0 (the Roots default) the hook's
stubs pass nothing to ignore, as stock's code does, and the ghost over the construction ship is
**green all the same**: QOL.log says `clear` without the `(the construction ship ignored)` that
the key at 1 adds, and the blueprint is not red. Roots' own placement (`CraftClass::CanPlaceHere`
is FleetOpsHook's port, and `Is_Clear_For_Placement` has Roots' own edit) does not count the
ship that is building as a blocker, so `RigIgnoresSelf` has nothing to add there and is off in
the Roots defaults. Then the ghost is moved over the other Mediterranean class ship
(about 465,760), to see whether Roots' ghost goes red over a ship at all.

1. Launch the game.
2. Wait for the main menu.
3. Expect "QOL.log" contains "build armada2/fo-roots-5.0Z11".
4. Expect "QOL.log" contains "RigIgnoresSelf=0  (a construction ship blocks its own blueprint, as in stock)".
5. Include "_fo-sides".
6. Click "LAUNCH".
7. Wait up to 180 seconds for "6000".
8. Wait 5 seconds.
9. Move the mouse to 335,640.
10. Wait 1 second.
11. Click at 335,640.
12. Wait 2 seconds.
13. Move the mouse to 599,787.
14. Wait 1 second.
15. Click at 599,787.
16. Wait 2 seconds.
17. Move the mouse to 459,787.
18. Wait 1 second.
19. Click at 459,787.
20. Wait 1 second.
21. Move the mouse to 336,700.
22. Wait 1 second.
23. Move the mouse to 336,640.
24. Wait up to 20 seconds for "QOL.log" to say "the blueprint's spot is clear".
25. Take a screenshot called "ghost over the ship, key off".
26. Check that the large translucent blueprint of a shipyard over the Mediterranean class ship at the left is tinted green, not red.
27. Move the mouse to 560,700.
28. Wait 3 seconds.
29. Take a screenshot called "ghost over the other ship".
30. Note "what colour the ghost has over the other ship is the finding: red would be Roots counting other ships as blockers"
31. Expect "QOL.log" not contain "the construction ship ignored".
32. Expect "QOL.log" not contain "patch refused".
33. Quit the game.
34. Expect the game to have exited.
35. Expect no crash.
