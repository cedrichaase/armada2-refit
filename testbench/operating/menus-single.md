# Operating the single-player screens with the UI's specs (`scenarios/ui-menus-single.md`)

What was learned driving the campaign selection, mission list, Save/Load, Tech Tree and
Admiral's Log under `ui/bench-asi.sh --menus` (`ui/MENUS.md`, "The single-player specs").

- **Reach a mission fast**: main menu column "Single Player" (1280x720: click 228,258),
  `Click "Federation Campaign"` (the card's title), `Click "Invasion"`, `Click "O.K."`,
  `Wait up to 300 seconds for the briefing`, `Click "OK"`. About two minutes. For a
  mission with no campaign, `drive launch -nointro a2_borg01` (30 s) also reaches the
  in-mission menu, Save, Load, the tech tree and the log.
- **A running game does not pick up a rebuilt `UI.asi`.** To iterate on a spec: end only
  this session's game (find the `Armada2.exe` processes whose environment names the
  session; never `pkill -f Armada2.exe`), run `ui/bench-asi.sh <state.json> --menus`
  again (it rebuilds and copies), `drive launch -nointro`. About 40 s to the main menu.
- **In-mission menu, 1280x720, the specs' places**: Esc, `Click "Save Game"`; Save's name
  field at 700,607, its Save button 640,639, Previous Menu 368,639; the list's rows from
  y=111, 28 px apart; the message dialogs ("Save Successful", the abort confirmation) are
  still the generic mirrored screen: O.K. at 638,421, Yes 505,428 and No 773,428. Admiral's
  Log: tabs along y=85 (Score 132, Military 278, Economy 422, Timeline 567, Battles 712,
  Ships 857, Build 1002, Tally 1147), Done 1132,665. Technology Tree's OK 640,652.
- **A quick `click` can come before the screen is up**: wait 2 to 3 s after a click that
  opens a dialog before the next one.
- **Type**: a text field of ours takes `Press End`, `Press BackSpace N times` and `Type`
  like a stock edit; `ctrl+a` selects its whole text.
- **`drive` with shell variables or heredocs may be refused by the agent sandbox**: put the
  steps in small script files (`d.sh` wrapping `a2test drive --session ID "$@"`).
