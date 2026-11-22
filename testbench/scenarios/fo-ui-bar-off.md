# Fleet Operations: Roots with the UI: the button bar left to Roots (the A side of fo-ui-bar: the same steps with HUD.Bar=0)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Bar=0
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`, beside `fo-ui-bar`: the same steps with the bar
left to Roots (the UI's `HUD.Bar=0`, Roots' default), to tell what the bench's pointer does on its own
from what the skin does (FO-6 step 6, decision 128). The log still has the layout and hover lines.

1. Include "_fo-render3d-skirmish".
2. Move the mouse to 700,300.
3. Wait 1 second.
4. Select the Federation construction ship (Mediterranean class) alone with a box selection, a small box dragged round that one ship only and never a click (a box is made with the drive step command, `a2test drive --session ID step "Drag from 300,590 to 385,695"`; hover a ship to read its class in the stock tooltip; the Starbase is the big station), then, as your last action, run the step "Move the mouse to 700,300" so that the pointer ends exactly there (the bench aims its later moves from that place), and stop once the bar shows eight buttons.
5. Expect "UI.log" not contain "bar: ours".
6. Expect "UI.log" contains "7:Weapon:npc_tractorbeam".
7. Expect "UI.log" contains "bar: measure mode=1".
8. Expect "UI.log" contains "use=14x20 floating=3x7".
9. Take a screenshot called "construction ship selected".
10. Wait 1 second.
11. Wait 1 second.
12. Move the mouse to 810,788.
13. Wait 1 second.
14. Move the mouse to 814,790.
15. Wait up to 10 seconds for "UI.log" to say "hover slot 7 Weapon npc_tractorbeam".
16. Wait 2 seconds.
17. Take a screenshot called "tractor beam hover".
18. Check that a thin red circle is drawn on the map round the selected ship, that the Tractor Beam tooltip (400 Range) is drawn beside the pointer, and that the Tractor Beam button is drawn lit.
19. Move the mouse to 700,300.
20. Wait 1 second.
21. Move the mouse to 320,788.
22. Wait 1 second.
23. Move the mouse to 324,790.
24. Wait up to 10 seconds for "UI.log" to say "hover slot 0 Command repair".
25. Wait 2 seconds.
26. Take a screenshot called "repair hover, no ring".
27. Check that no red circle is drawn on the map and the repair tooltip shows.
28. Move the mouse to 700,300.
29. Wait 1 second.
30. Move the mouse to 600,788.
31. Wait 1 second.
32. Move the mouse to 604,790.
33. Wait 1 second.
34. Click at 604,790.
35. Wait up to 20 seconds for "UI.log" to say "Build:fed_outpost".
36. Expect "UI.log" contains "Command:cancel".
37. Wait 2 seconds.
38. Take a screenshot called "build list".
39. Check that Roots' own bar shows the build list: the buildings' buttons, some of them dimmed or red, and a back button last.
40. Move the mouse to 700,300.
41. Wait 1 second.
42. Move the mouse to 1015,788.
43. Wait 1 second.
44. Move the mouse to 1020,790.
45. Wait 1 second.
46. Click at 1020,790.
47. Wait up to 20 seconds for "UI.log" to say "7:Weapon:npc_tractorbeam".
48. Wait 2 seconds.
49. Take a screenshot called "back to the top level".
50. Check that Roots' own bar shows the ship's eight commands again.
51. Expect "UI.log" not contain "standing down: the bar".
52. Expect "UI.log" not contain "PANIC".
53. Expect "Display8.log" not contain "VUID".
54. Expect "Display8.log" not contain "device removed".
55. Quit the game.
56. Expect the game to have exited.
57. Expect no crash.
