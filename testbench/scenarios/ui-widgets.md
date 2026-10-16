# The UI runtime's widgets: text field, list, scroll, dropdown, tabs, tooltip, modal (UI.asi)

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --demo shell
Timeout: 10 min

`ui/README.md`, "Widgets". `UI.asi` is not in `./install` yet; the setup puts it in the
clone with the demonstration screen on. In the front end the Credits are replaced by a
screen of ours (`[Demo] Shell=1`): the old demo panel at the left and, at the right, the
widget gallery (`ui/src/gallery.rs`), four tabs: Text, Lists, Choice and Misc. Positions
below are screen pixels at 1280x720 (the gallery is laid out in units of a 1080-line screen,
so they move with the resolution; this scenario is pinned to one). Esc or Enter leave
the screen only when no widget of ours holds the keyboard; a right click always does.

The bench types lower case only (`a2input type` sends no Shift), so the typed text is
lower case. Wheel steps are `a2input wheel`, positive down.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "shell: hooked".
4. Click the credits.
5. Wait 3 seconds.
6. Take a screenshot called "gallery".
7. Expect "WIDGET GALLERY" is visible.
8. Note "text field: click in it, type, select with Shift+arrows, delete, Enter submits".
9. Click at 985,227.
10. Type "hello".
11. Expect "Typed: hello" is visible.
12. Press shift+Left 2 times.
13. Press BackSpace.
14. Expect "Typed: hel" is visible.
15. Press Return.
16. Expect "Submitted: hel" is visible.
17. Note "Ctrl+A, Ctrl+C, Tab to the next field, Ctrl+V: the clipboard, and the keyboard focus moving".
18. Press ctrl+a.
19. Press ctrl+c.
20. Press Tab.
21. Press ctrl+v.
22. Expect "Typed: hel / hel" is visible.
23. Take a screenshot called "text".
24. Note "number spinner: the plus button, held to repeat".
25. Click at 971,372.
26. Expect "Count 4" is visible.
27. Click "Lists".
28. Wait 1 second.
29. Note "list: click selects, Down moves, double-click activates, End goes to the last row".
30. Click at 800,252.
31. Expect "Selected: Defiant" is visible.
32. Press Down.
33. Expect "Selected: Akira" is visible.
34. Double-click at 800,252.
35. Expect "Activated: Defiant" is visible.
36. Press End.
37. Expect "Selected: Tactical" is visible.
38. Note "wheel over the scroll area at the right: the first lines scroll away, the later ones come in".
39. Move the mouse to 1100,300.
40. Scroll the wheel down 5 times.
41. Wait 1 second.
42. Expect "Scrolled line 16" is visible.
43. Take a screenshot called "lists".
44. Click "Choice".
45. Wait 1 second.
46. Note "dropdown: opens a list over everything below it, a click on a row picks it".
47. Click at 850,206.
48. Wait 1 second.
49. Take a screenshot called "dropdown open".
50. Click "Klingon".
51. Expect "Faction: Klingon" is visible.
52. Note "check box and radio button".
53. Click at 736,298.
54. Expect "Cloak true" is visible.
55. Click "Defensive".
56. Expect "Stance 1" is visible.
57. Note "image button with the game's own cursor art, and a tooltip after the pointer rests on it".
58. Move the mouse to 746,440.
59. Wait 2 seconds.
60. Take a screenshot called "tooltip".
61. Expect "An image button" is visible.
62. Click "Misc".
63. Wait 1 second.
64. Note "modal: dims everything, takes all input; a click outside does nothing, Esc closes".
65. Click "Open modal".
66. Wait 1 second.
67. Take a screenshot called "modal".
68. Expect "A MODAL" is visible.
69. Click at 1000,500.
70. Wait 1 second.
71. Expect "A MODAL" is visible.
72. Press Escape.
73. Wait 1 second.
74. Expect "Modal closed 1 times" is visible.
75. Press Escape.
76. Wait for the main menu.
77. Expect "UI.log" contains "demo screen left".
78. Check that the screenshots "gallery", "dropdown open", "tooltip" and "modal" show the widgets drawn cleanly: the dropdown's list over the rows below it, the tooltip on screen beside the pointer, the modal's dimming over both panels, and nothing from the game's own menus.
