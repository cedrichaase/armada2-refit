# Round-3 fixes to the UI scenarios: what was learned

For the agents after (`ui-hud-map`, `ui-hud-panels`, `ui-hud-panels-objectives`,
`ui-loading`, `ui-runtime`).

- **The base's clearing on Warzone is small.** At the start of a Borg Instant Action
  the minimap is nearly all shroud: the clearing is a patch about an eighth of the
  square across at the start position, with the base's blips (white for the Borg
  player) and the red nebula it sits in (magenta on the map picture). The stock
  minimap shows the same; a judge told to expect "a cleared area with dots" calls it
  all shroud. `UI.log`'s "minimap: shroud N of 16384 cells clear" is the check.
- **Instant Action is not single player to the engine**: its transport
  (`TransportLocalInstantAction`) answers false to `IsSinglePlayer`. Anything that
  tells a campaign from a network game must ask `IsInstantAction` first.
- **The Academy's "User Interface" tutorial starts with zooming**, not selecting:
  lesson 1's objectives are "Zoom your view in or out by scrolling the mouse wheel",
  "Click the OK button or press the 'Enter' key ..." and "After you have completed
  this lesson, press the 'Enter' key to advance". Enter does not advance until the
  wheel has turned; lesson 2 is "Select the Enterprise".
- **The chat is themed by race in a mission**: with `HUD.Chat=1` our history box and
  our field wear the race theme (green for the Borg), so a judged step must not tell
  ours from stock's by "green".
