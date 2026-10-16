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
7. Check that both panels are drawn cleanly over the backdrop, with nothing from the game's own menus showing.
8. Expect "WIDGET GALLERY" is visible.
9. Note "text field: click in it, type, select with Shift+arrows, delete, Enter submits".
10. Click at 985,227.
11. Type "hello".
12. Expect "Typed: hello" is visible.
13. Press shift+Left 2 times.
14. Press BackSpace.
15. Expect "Typed: hel" is visible.
16. Press Return.
17. Expect "Submitted: hel" is visible.
18. Note "Ctrl+A, Ctrl+C, Tab to the next field, Ctrl+V: the clipboard, and the keyboard focus moving".
19. Press ctrl+a.
20. Press ctrl+c.
21. Press Tab.
22. Press ctrl+v.
23. Expect "Typed: hel / hel" is visible.
24. Take a screenshot called "text".
25. Note "number spinner: the plus button, held to repeat".
26. Click at 971,372.
27. Expect "Count 4" is visible.
28. Click "Lists".
29. Wait 1 second.
30. Note "list: click selects, Down moves, double-click activates, End goes to the last row".
31. Click at 800,252.
32. Expect "Selected: Defiant" is visible.
33. Press Down.
34. Expect "Selected: Akira" is visible.
35. Double-click at 800,252.
36. Expect "Activated: Defiant" is visible.
37. Press End.
38. Expect "Selected: Tactical" is visible.
39. Note "wheel over the scroll area at the right: the first lines scroll away, the later ones come in".
40. Move the mouse to 1100,300.
41. Scroll the wheel down 5 times.
42. Wait 1 second.
43. Expect "Scrolled line 16" is visible.
44. Take a screenshot called "lists".
45. Click "Choice".
46. Wait 1 second.
47. Note "dropdown: opens a list over everything below it, a click on a row picks it".
48. Click at 850,206.
49. Wait 1 second.
50. Take a screenshot called "dropdown open".
51. Check that the open dropdown's list is drawn over the rows below it.
52. Click "Klingon".
53. Expect "Faction: Klingon" is visible.
54. Note "check box and radio button".
55. Click at 736,298.
56. Expect "Cloak true" is visible.
57. Click "Defensive".
58. Expect "Stance 1" is visible.
59. Note "image button with the game's own cursor art, and a tooltip after the pointer rests on it".
60. Move the mouse to 746,440.
61. Wait 2 seconds.
62. Take a screenshot called "tooltip".
63. Check that a tooltip is on screen beside the pointer.
64. Expect "An image button" is visible.
65. Click "Misc".
66. Wait 1 second.
67. Note "modal: dims everything, takes all input; a click outside does nothing, Esc closes".
68. Click "Open modal".
69. Wait 1 second.
70. Take a screenshot called "modal".
71. Check that the modal is drawn over both panels, which are dimmed beneath it.
72. Expect "A MODAL" is visible.
73. Click at 1000,500.
74. Wait 1 second.
75. Expect "A MODAL" is visible.
76. Press Escape.
77. Wait 1 second.
78. Expect "Modal closed 1 times" is visible.
79. Press Escape.
80. Wait for the main menu.
81. Expect "UI.log" contains "demo screen left".
