# A mission won and a mission lost, with the UI's menus and loading screen

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus Loading.Enable=1 --scene ending
Timeout: 25 min

`ui/MENUS.md`, "The single-player specs": there is no end-of-mission screen to host.
`do_success_mission` only marks the campaign's next mission as reached; after a won
campaign mission the game loads the next one at once (its cinematic, then the loading
screen, ours here), and after a lost one it goes back to the shell. The first Federation
mission is entered through the UI's menus as `ui-menus-single.md` does, and ended by
`Scene.asi`'s `succeed` (`ScriptInterfaceImp::SucceedMission`, the script's own win;
`SCENE=ending` adds nothing to the mission). The second mission, "Defiance" in the
Federation's list, loads with the loading screen of ours naming it; there `fail` ends it
as lost.

Clicks that are not on a label are by place (1280x720 only): the main menu's Single
Player.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Expect "UI.log" contains "loading: a screen of ours".
5. Click at 228,258.
6. Wait 3 seconds.
7. Click "Federation Campaign".
8. Wait 3 seconds.
9. Click "Invasion".
10. Wait 1 second.
11. Click "O.K.".
12. Wait up to 300 seconds for the briefing.
13. Wait 3 seconds.
14. Click "OK".
15. Wait 10 seconds.
16. Expect "UI.log" contains "a2_fed01".
17. Scene "succeed".
18. Wait 5 seconds.
19. Take a screenshot called "won, 5 s".
20. Wait 20 seconds.
21. Take a screenshot called "won, 25 s".
22. Wait 60 seconds.
23. Take a screenshot called "won, 85 s".
24. Wait up to 300 seconds for the briefing.
25. Expect "UI.log" contains "a2_fed02".
26. Take a screenshot called "next mission".
27. Wait 3 seconds.
28. Click "OK".
29. Wait 10 seconds.
30. Scene "fail".
31. Wait 5 seconds.
32. Take a screenshot called "lost, 5 s".
33. Wait 20 seconds.
34. Take a screenshot called "lost, 25 s".
35. Quit the game.
36. Expect no crash.
