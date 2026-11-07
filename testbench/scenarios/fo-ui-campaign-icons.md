# Fleet Operations: Roots with the UI: the campaign emblems on Single Player are the factions' logos

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 10 min

Run with `a2test --game fo-roots` and `--install .`. The column's four emblems are the
faction logos the fleetops content set names (`[Campaign.*] Emblem =`, PLAN-FLEETOPS.md,
decision 530; ui/MENUS.md, "Campaign emblems from a picture"), read from Roots' Textures.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Move the mouse to 600,300.
5. Wait 1 second.
6. Click "Single Player".
7. Wait up to 30 seconds for "UI.log" to say "adopted at".
8. Wait 4 seconds.
9. Take a screenshot called "single".
10. Expect "UI.log" contains "menus: campaign Allied emblem textures\federation_logo.tga: read".
11. Expect "UI.log" contains "menus: campaign FreeBorg emblem textures\free_borg_logo.tga: read".
12. Check that the Single Player column lists four campaigns, each with a small coloured faction logo beside its name (a wreathed blue globe, a green diamond, an orange alien head, a blue X), not a cut of a picture.
