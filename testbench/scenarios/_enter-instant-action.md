# Fragment: from launch into an Instant Action match, as the Borg, against one easy AI

Included by scenarios that need a match with units of a KNOWN faction: a Nexus, two
Assemblers and a Resource Collector at the start, with Resources: Unlimited (the
Instant Action default, so every counter reads 999999). A direct map launch
(`Launch: -nointro a2_borg01`) is no substitute: it opens a map with no units at all.

What these steps encode, so nobody has to find it again. One player is not enough:
LAUNCH with every other slot empty answers "Launch Failed: The game cannot be launched
with only one player." Slot 2's name dropdown offers Empty Slot, Computer AI:Easy,
AI:Medium and AI:Hard; Easy leaves you alone. The Side dropdown opens over the slots
below it (Federation, Borg, Romulan, Klingon, Cardassian, Species 8472, Random); Random
deals any race, and panels and units differ per race, so pick one. The match is up
about 5 seconds after LAUNCH (map Warzone, the default). Game Setup is the UI's
(`ui/MENUS.md`, the setup spec; `./install` no longer installs Menus.asi): slot 2's
name list is opened and chosen by its text; player 1's side list by place, `hud
767,203`, which is that field at 16:9 only (the spec's columns are fractions of the
screen's width; "Random" is in every row, so it cannot be clicked by its text). Every
scenario that includes this runs at 16:9. Before the UI hosted the menus, these were
design-space clicks into the stock screen (124,115 / 124,143 / 246,95 / 246,123).

(Prose only above: in a scenario every numbered or bulleted line is a step.)

1. Launch the game.
2. Wait for the main menu.
3. Click the instant action cube.
4. Wait for "GAME SETUP".
5. Click "Computer AI:Medium".
6. Wait 1 second.
7. Click "Computer AI:Easy".
8. Wait 1 second.
9. Click at hud 767,203.
10. Wait 1 second.
11. Click "Borg".
12. Wait 1 second.
13. Expect "Computer AI:Easy" is visible.
14. Expect "Borg" is visible.
15. Click "LAUNCH".
16. Wait 8 seconds.
