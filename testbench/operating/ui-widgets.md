# Operating the widget gallery and the cursor on the bench

What was learned driving `UI.asi`'s gallery (`ui/src/gallery.rs`) and cursor, for the
next agent. Scenarios: `scenarios/ui-widgets.md`, `scenarios/ui-widgets-cursor.md`.

- **A session needs `UI.asi` before the game starts.** `session start` launches the game
  at once; use `session start --no-launch`, then `ui/bench-asi.sh <state.json> --demo
  shell UI.Cursor=1` (the state file is in the session's `state:` line), then
  `drive launch` (or `drive launch -nointro a2_borg01` after `ui/bench-widgets.sh
  <state.json>` for the cursor scene), then `drive step 'Wait for the main menu'` and
  `drive step 'Click the credits'`. About 30 s from launch to the menu.
- **The gallery's positions** at 1280x720, in screen pixels: the tabs at y=168 (Text 790,
  Lists 922, Choice 1052, Misc 1182); on Text the name field at 985,227, the code field
  at 985,279, the two spinners' plus buttons at 971,372 and 1233,372; on Lists the list
  from 728,190 to 983,447 (rows 24 px apart from y=204) and the scroll area to its right;
  on Choice the faction dropdown at 850,206 and its list opening below; the Shields and
  Cloak boxes at 736,268 and 736,298; the image button at 746,440.
- **`click-text` finds a word the readouts also print.** `Click "Cloak"` hit the readout
  "Cloak false" and not the check box. Click by position when a label is also printed
  elsewhere on the screen.
- **There was no wheel step.** `scenario.py` now has `Scroll the wheel down|up [N times]`
  (a2input's `wheel`, positive = down, 0.15 s apart), also usable as `drive step`.
- **`a2input type` sends lower case**, whatever the string. `key shift+Left`, `ctrl+a`,
  `ctrl+c`, `ctrl+v`, `Tab`, `Return`, `Escape`, `Next` (PageDown), `End` all arrive.
  The clipboard works under Proton (Ctrl+C then Ctrl+V into a second field).
- **Shell loop and Esc/Enter.** The demo screen leaves on Esc or Enter unless a widget
  holds the keyboard (`input::keys_captured`): a focused text field gets them. A right
  click always leaves.
- **The cursor in the shell** hides Windows' arrow (`shell::hide_os_cursor`) when
  `Cursor=1`; the bench screenshots show only ours. In a mission the engine's is hidden by
  the `RefreshDisplay` wrap; `UI.log` names every shape change.
- **Sandbox**: an agent shell may refuse `drive step 'Click "X"'` inside loops or chains;
  put the sequence in a small script file and run `bash script`.
