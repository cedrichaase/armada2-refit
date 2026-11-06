# Fleet Operations: Roots with Render3D: the Dominion's start fleet is claimed and drawn (the races sweep)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 10 min

The hull sweep of 2026-11-06 (`research/fo-roots/hull-sweep`) played the start fleets of
every race on Roots. This runs one of them, the Dominion against the Federation AI (the
sides picked like `_fo-sides`: the player's list at its first row, items 21 px apart, the AI's
list the same from 144), and checks what the claim logs per hull: `hulls: <sod>: N of M
meshes drawn after the database (K nodes hidden by it)`, and `NOTHING DRAWN` for a hull
claimed with no mesh. The other races are the same steps with other list items (Borg
+20, Dominion +41, Federation +62, Klingon +83, Romulan +104, Cardassian +125,
Species 8472 +146, Free Borg +167, from the list's own row at 114).

1. Launch the game.
2. Wait for the main menu.
3. Move the mouse to 640,520.
4. Wait 1 second.
5. Click "Instant Action".
6. Wait up to 30 seconds for "LAUNCH".
7. Wait 3 seconds.
8. Move the mouse to shell 270,114.
9. Wait 1 second.
10. Click at shell 278,114.
11. Wait 2 seconds.
12. Move the mouse to shell 278,155.
13. Wait 1 second.
14. Click at shell 278,155.
15. Wait 2 seconds.
16. Move the mouse to shell 270,144.
17. Wait 1 second.
18. Click at shell 278,144.
19. Wait 2 seconds.
20. Move the mouse to shell 278,206.
21. Wait 1 second.
22. Click at shell 278,206.
23. Wait 2 seconds.
24. Expect "Dominion" is visible.
25. Click "LAUNCH".
26. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
27. Wait 6 seconds.
28. Press Home.
29. Wait 2 seconds.
30. Move the mouse to 640,400.
31. Scroll the wheel up 12 times.
32. Wait 2 seconds.
33. Take a screenshot called "near".
34. Press Home.
35. Wait 2 seconds.
36. Take a screenshot called "far".
37. Expect "Render3D.log" contains "dom_destroyer: levels of detail".
38. Expect "Render3D.log" contains "dom_worker_ship: 14 of 15 meshes drawn".
39. Expect "Render3D.log" not contain "NOTHING DRAWN".
40. Expect "Display8.log" not contain "VUID".
41. Expect "Display8.log" not contain "device removed".
42. Quit the game.
43. Expect the game to have exited.
44. Expect no crash.
