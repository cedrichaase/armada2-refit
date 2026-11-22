# Every icon of the Assembler's build list is drawn (issue #85)

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh
Timeout: 10 min

`ui/HUD.md`, "The button bar", "Icons". Four Borg build icons (`gbbsensor`,
`gbbturret`, `gbbturret2`, `gbbtechass`: the Detection Array, the Energy Cannon, the
Pulse Cannon and the Technology Assimilator) are 32-bit TGAs whose alpha is 0 in every
texel; the stock bar draws them opaque, and the UI's bar once drew them fully
transparent, leaving four blank cells (W, Z, C and F in the build list: the fill order
runs down the left columns). `gfx::load_tga` now draws such a file opaque and logs it
("tga: textures\gbbsensor.tga has no alpha; drawn opaque").

The match and the clicks are `ui-hud-bar.md`'s: Borg, Warzone, Home to put the camera
on the base, the first Assembler at about 640,175 and the bar's build button (Q) at
about 250,598 at 1280x720.

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "bar: ours".
3. Press Home.
4. Wait 2 seconds.
5. Click at 640,175.
6. Wait up to 5 seconds for the logs the next steps expect.
7. Expect "UI.log" contains "| Assembler |".
8. Click at 250,598.
9. Wait 1 second.
10. Expect "UI.log" contains "bar: click build".
11. Move the mouse to 1000,150.
12. Wait 1 second.
13. Expect "UI.log" contains "gbbsensor.tga has no alpha; drawn opaque".
14. Expect "UI.log" contains "gbbturret.tga has no alpha; drawn opaque".
15. Expect "UI.log" contains "gbbturret2.tga has no alpha; drawn opaque".
16. Expect "UI.log" contains "gbbtechass.tga has no alpha; drawn opaque".
17. Take a screenshot called "build menu".
18. Check that the button grid at the bottom left shows the Borg build list with a picture of a station in every cell of the four left columns (thirteen pictures: none of those twelve cells, nor the bottom cell of the fifth column, is an empty dark square), a cancel sign at the top of the fifth column and a back arrow below it.
19. Quit the game.
20. Expect no crash.
