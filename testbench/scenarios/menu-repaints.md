# What a menu draws last reaches the screen; the Technology Tree keeps its fixed pitch

Resolutions: 21:9
Mod: refit
Timeout: 12 min

Regression test for menus 4.3.1, three findings of the 2026-10-02 bench sweep.

Options' version label "1.1" is the last thing its WM_PAINT draws, and the Manual IP
field is an edit that paints itself last. Embedded under the 3D window, Wine kept both
in the window surface and never pushed them to the screen: the label never appeared,
and the field was a bare black panel until it was clicked. The "manual ip" shot is for
the eye -- the field is empty, so there is nothing for OCR to find until an address is
typed.

The Technology Tree's edit is raster Courier; held to TrueType by name alone it came
back proportional, and its ASCII branches no longer lined up. "Mining Freighter" sits
at the end of an indented branch, so where its centre lands measures the pitch: about
design x 266 in a fixed-pitch font the width of stock's, about 222 in the proportional
one.

1. Launch the game.
2. Wait for the main menu.
3. Click the options panel.
4. Wait for "Sound Settings".
5. Take a screenshot called "options".
6. Expect "1.1" is visible inside design 300,505,200,35 and at least 8 design px tall.
7. Click "Previous Menu".
8. Wait for the main menu.
9. Click the multiplayer turbolift.
10. Wait for the multiplayer connection screen.
11. Click "Internet - Manual IP".
12. Wait 3 seconds.
13. Take a screenshot called "manual ip".
14. Click at design 400,292.
15. Type "10.0.0.1".
16. Wait 1 second.
17. Take a screenshot called "address typed".
18. Expect "10.0.0.1" is visible inside design 236,279,316,25 and at least 8 design px tall.
19. Press Escape.
20. Wait 2 seconds.
21. Click "Previous Menu".
22. Wait for the main menu.
23. Click the single player emblem.
24. Wait for the single player screen.
25. Click the Federation campaign.
26. Wait for "Invasion".
27. Click the first mission.
28. Click the mission OK button.
29. Wait up to 300 seconds for the briefing.
30. Wait 3 seconds.
31. Click "OK".
32. Wait 2 seconds.
33. Press Escape.
34. Wait for the options menu.
35. Expect "1.1" is visible inside design 300,505,200,35 and at least 8 design px tall.
36. Click "Technology Tree".
37. Wait for "Orbital Processing Facility".
38. Take a screenshot called "technology tree".
39. Expect "Mining Freighter" is visible inside design 240,212,60,24.
40. Click "OK".
41. Wait for the options menu.
42. Click "Return to Game".
43. Wait 2 seconds.
44. Quit the game.
45. Expect no crash.
