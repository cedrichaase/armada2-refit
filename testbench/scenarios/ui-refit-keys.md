# The Refit screen's Keys tab: listing, searching and rebinding Input.map's keys and the grid's

Resolutions: 1280x720
Mod: refit
Timeout: 20 min

`ui/MENUS.md`, "The Keys tab". Run it with `--install .` so that the clone has the
checkout's UI.asi and QOL.asi. Not yet driven on the bench: the places of the tab's rows
are not measured, so every click is by label (the help strip says nothing until the
pointer rests on a row, so the first OCR hit of a label is its row).

**Listing and search.** The Keys tab opens on the grid; *Camera* lists *Zoom camera in*
with stock's Ctrl+Up; the search field finds *Reset zoom* from "zoom".

**Rebinding a key of Input.map.** A click on *Zoom camera in* and a press of F6 write
`[UI.Keys] zoom_factor_plus=F6` to the overlay, rewrite Input.map beside its backup, and
the row says *Restart needed*. Relaunched, UI.log reads the stock from the backup and one
key of the player's.

**Rebinding a grid key.** A click on the grid's third button of the top row and F1
rebind it at once: QOL.log `bar: Cell3 E -> F1`.

**Reset.** Reset tab removes both keys; Input.map is stock again and the backup goes.

## Open the tab
1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "keys: Input.map (stock): ".
4. Expect "QOL.log" contains "bar: keys Q W E R T / A S D F G / Z X C V B".
5. Click at 228,390.
6. Wait 3 seconds.
7. Click "Refit settings".
8. Wait for the refit screen.
9. Click "Keys".
10. Wait 2 seconds.
11. Expect "Command bar grid" is visible.
12. Take a screenshot called "keys grid".
## Camera and search
13. Click "Camera".
14. Wait 1 second.
15. Expect "Zoom camera in" is visible.
16. Expect "Ctrl+Up" is visible.
17. Take a screenshot called "keys camera".
18. Click "Search keys".
19. Type "zoom".
20. Wait 1 second.
21. Expect "Reset zoom" is visible.
22. Press Escape.
23. Wait 1 second.
24. Expect "UI.log" not contain "refit: closed".
## Rebind a key of Input.map
25. Click "Camera".
26. Wait 1 second.
27. Click "Zoom camera in".
28. Wait 1 second.
29. Expect "Press the new key for" is visible.
30. Expect "UI.log" contains "refit: keys: press a key for zoom_factor_plus".
31. Press F6.
32. Wait 2 seconds.
33. Expect "UI.log" contains "keys: zoom_factor_plus Ctrl+Up -> F6 (restart)".
34. Expect "UI.log" contains "keys: Input.map written: 1 key of yours".
35. Expect "RefitPlayer.ini" contains "zoom_factor_plus=F6".
36. Expect "Restart needed" is visible.
37. Take a screenshot called "keys rebound".
## Rebind a grid key
38. Click "Command bar grid".
39. Wait 1 second.
40. Click "Top row, 3rd button".
41. Wait 1 second.
42. Press F1.
43. Wait 2 seconds.
44. Expect "UI.log" contains "keys: grid Cell3 E -> F1".
45. Expect "QOL.log" contains "bar: Cell3 E -> F1".
46. Expect "RefitPlayer.ini" contains "Cell3=F1".
## Relaunch: the overlay's key is in Input.map
47. Click "Done".
48. Wait 1 second.
49. Quit the game.
50. Launch the game.
51. Wait for the main menu.
52. Expect "UI.log" contains "keys: Input.map (stock from the backup): ".
53. Expect "UI.log" contains "1 of yours".
54. Expect "QOL.log" contains "bar: keys Q W F1 R T".
## Reset
55. Click at 228,390.
56. Wait 3 seconds.
57. Click "Refit settings".
58. Wait for the refit screen.
59. Click "Keys".
60. Wait 2 seconds.
61. Click "Reset tab".
62. Wait 1 second.
63. Click "Yes".
64. Wait 2 seconds.
65. Expect "UI.log" contains "refit: Keys reset (2 keys removed)".
66. Expect "UI.log" contains "keys: Input.map is stock again".
67. Expect "RefitPlayer.ini" not contain "zoom_factor_plus=".
68. Expect "QOL.log" contains "bar: Cell3 F1 -> E".
69. Take a screenshot called "keys reset".
