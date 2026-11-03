# Every plugin on its symbol profile: the build recognised, every symbol resolved

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh stub online/bench-asi.sh
Timeout: 20 min

Run with `--install .`. On GOG's 1.1 every plugin that has moved onto its symbol
profile (`crates/a2engine/README.md`) logs the executable's identity, recognises the
build, resolves all of its symbols with no hook blocked, and no hook stands down or
has a symbol read before its hook was ready. One game straight into a mission, so the
hooks that are readied late (the stub's mode requests at the first `Direct3DCreate8`,
`[Drop]`'s ranges at the engine's first scene) have run. Each package of phase 8
appends its plugin's section and touches no other.

## Mission
1. Launch the game.
2. Wait 60 seconds.
3. Wait 20 seconds.
4. Take a screenshot called "mission".
5. Check that the screenshot shows a Star Trek: Armada II mission (its HUD around the map view; with no scene staged the 3D view may be empty), not a black or garbled screen.

## Display8
6. Expect "Display8.log" contains "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321".
7. Expect "Display8.log" contains "engine: build armada2/gog-1.1".
8. Expect "Display8.log" contains "engine: symbols: 15/15 resolved; 0 hooks blocked".
9. Expect "Display8.log" contains "stub: the engine's own mode requests (6 of 6 calls)".
10. Expect "Display8.log" not contains "stands down: ".
11. Expect "Display8.log" not contains "read before its hook".
12. Expect "Display8.log" not contains "engine: TEST:".
13. Expect "Display8.log" not contains "PANIC".

## UI
14. Wait up to 60 seconds for "UI.log" to say "hud: panels of ours".
15. Expect "UI.log" contains "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321".
16. Expect "UI.log" contains "engine: build armada2/gog-1.1".
17. Expect "UI.log" contains "engine: symbols: 54/54 resolved; 0 hooks blocked".
18. Expect "UI.log" contains "engine: loading symbols: 17/17 resolved; 0 hooks blocked".
19. Expect "UI.log" contains "engine: radar symbols: 116/116 resolved; 0 hooks blocked".
20. Expect "UI.log" contains "engine: info symbols: 137/137 resolved; 0 hooks blocked".
21. Expect "UI.log" contains "engine: hud symbols: 169/169 resolved; 0 hooks blocked".
22. Expect "UI.log" contains "engine: panels symbols: 175/175 resolved; 0 hooks blocked".
23. Expect "UI.log" contains "engine: shell symbols: 92/92 resolved; 0 hooks blocked".
24. Expect "UI.log" contains "engine: dialogs symbols: 39/39 resolved; 0 hooks blocked".
25. Expect "UI.log" not contains "stands down: ".
26. Expect "UI.log" not contains "read before its hook".
27. Expect "UI.log" not contains "engine: TEST:".
28. Expect "UI.log" not contains "PANIC".

## BinkProxy
29. Expect "BinkProxy.log" contains "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321".
30. Expect "BinkProxy.log" contains "engine: build armada2/gog-1.1".
31. Expect "BinkProxy.log" contains "engine: symbols: 6/6 resolved; 0 hooks blocked".
32. Expect "BinkProxy.log" not contains "stands down: ".
33. Expect "BinkProxy.log" not contains "read before its hook".
34. Expect "BinkProxy.log" not contains "engine: TEST:".

## Online
35. Expect "Online.log" contains "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321".
36. Expect "Online.log" contains "engine: build armada2/gog-1.1".
37. Expect "Online.log" contains "engine: symbols: 13/13 resolved; 0 hooks blocked".
38. Expect "Online.log" not contains "stands down: ".
39. Expect "Online.log" not contains "read before its hook".
40. Expect "Online.log" not contains "engine: TEST:".
41. Expect "Online.log" contains "menu entry: LAN (IPX) is Internet - Online".

## QOL
42. Expect "QOL.log" contains "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321".
43. Expect "QOL.log" contains "engine: build armada2/gog-1.1".
44. Expect "QOL.log" contains "engine: symbols: 133/133 resolved; 0 hooks blocked".
45. Expect "QOL.log" not contains "stands down: ".
46. Expect "QOL.log" not contains "read before its hook".
47. Expect "QOL.log" not contains "engine: TEST:".

## QOLRules
48. Expect "QOLRules.log" contains "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321".
49. Expect "QOLRules.log" contains "engine: build armada2/gog-1.1".
50. Expect "QOLRules.log" contains "engine: symbols: 63/63 resolved; 0 hooks blocked".
51. Expect "QOLRules.log" not contains "stands down: ".
52. Expect "QOLRules.log" not contains "read before its hook".
53. Expect "QOLRules.log" not contains "engine: TEST:".

## Render3D
54. Expect "Render3D.log" contains "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321".
55. Expect "Render3D.log" contains "engine: build armada2/gog-1.1".
56. Expect "Render3D.log" contains "engine: symbols: 72/72 resolved; 0 hooks blocked".
57. Expect "Render3D.log" contains "engine: world symbols: 26/26 resolved; 0 hooks blocked".
58. Expect "Render3D.log" contains "engine: particles symbols: 14/14 resolved; 0 hooks blocked".
59. Expect "Render3D.log" contains "engine: hulls symbols: 34/34 resolved; 0 hooks blocked".
60. Expect "Render3D.log" contains "engine: effects symbols: 53/53 resolved; 0 hooks blocked".
61. Expect "Render3D.log" contains "engine: ordnance symbols: 23/23 resolved; 0 hooks blocked".
62. Expect "Render3D.log" contains "engine: specials symbols: 10/10 resolved; 0 hooks blocked".
63. Expect "Render3D.log" contains "engine: sky symbols: 11/11 resolved; 0 hooks blocked".
64. Expect "Render3D.log" contains "engine: overlays symbols: 42/42 resolved; 0 hooks blocked".
65. Expect "Render3D.log" not contains "stands down: ".
66. Expect "Render3D.log" not contains "read before its hook".
67. Expect "Render3D.log" not contains "engine: TEST:".
68. Expect "Render3D.log" contains "hooks: frame the 3D views (the seam) at Armada_RenderFrame".

## End
69. Quit the game.
70. Expect no crash.
