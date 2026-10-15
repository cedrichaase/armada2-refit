# render3d: the build ghost, height posts, holograms and the Evolver

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 12 min

`render3d/README.md`, "hulls" ("Holograms", "The Evolver", "The build ghost") and
"overlays" ("Height posts"). Run it on a checkout: `./a2test run render3d-hulls-extras
--install . --res 1280x720`. The `stations` scene is the player's (team 1, Federation)
yards, a research station and two ships. A construction ship (`fconst`) is spawned and
selected; the button bar's Q (hud 403,983) opens its build menu and, clicked again,
picks the starbase (`fbase`), whose ghost then follows the pointer over open space
right of the yards, drawn by the hulls module as a faded hull with its height post
(the post is short: the ghost stands on the plane). A Romulan `rspecial` fires its
holo-emitter (`gholoemt`, three times: a shot does not always take) at a Klingon
battleship, which then shows as a hologram of the Romulan's class. An 8472 Evolver
(`8472_passive`) of the player's evolves into a Transmuter (Q, Q on its menu): its
cocoon (`8472_cocoon.sod`) grows, bursts, and the Transmuter comes out.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "height posts: 5 of 5 calls ours".
5. Note "the build ghost".
6. Scene "spawn con fconst 1700 0 800".
7. Wait 1 second.
8. Scene "select con".
9. Wait 1 second.
10. Click at hud 403,983.
11. Wait 1 second.
12. Click at hud 403,983.
13. Wait 1 second.
14. Move the mouse to hud 1000,717.
15. Wait 1 second.
16. Move the mouse to hud 1717,867.
17. Wait 1 second.
18. Scene "cursor off; camera 2730 350 700 2730 0 1080".
19. Wait 2 seconds.
20. Take a screenshot called "ghost".
21. Expect "Render3D.log" contains "hulls: build ghost of fbase".
22. Right-click at hud 1067,333.
23. Note "a hologram".
24. Scene "spawn rom rspecial 3000 0 3000 0 1; spawn foe kbattle 3300 0 3000 0 2; weapons foe off".
25. Scene "special rom gholoemt foe".
26. Wait 3 seconds.
27. Scene "special rom gholoemt foe".
28. Wait 3 seconds.
29. Scene "special rom gholoemt foe".
30. Scene "orbit foe 200 25 120".
31. Wait 6 seconds.
32. Take a screenshot called "hologram".
33. Expect "Render3D.log" contains "shows a hologram of rspecial".
34. Note "the Evolver".
35. Scene "camera rts; spawn evo 8472_passive 3400 0 2600 0 1".
36. Wait 1 second.
37. Scene "select evo; center evo".
38. Wait 1 second.
39. Click at hud 403,983.
40. Wait 1 second.
41. Click at hud 403,983.
42. Wait 1 second.
43. Move the mouse to hud 1067,333.
44. Wait 1 second.
45. Scene "camera 3400 120 2400 3400 0 2600".
46. Wait 3 seconds.
47. Take a screenshot called "cocoon".
48. Wait 15 seconds.
49. Take a screenshot called "transmuter".
50. Expect "Render3D.log" contains "an Evolver's cocoon".
51. Expect "Render3D.log" not contains "survey: left to the engine".
52. Expect "Render3D.log" not contains "PANIC".
53. Expect the game is still running.
54. Quit the game.
55. Expect no crash.
