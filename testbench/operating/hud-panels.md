# The UI's resources, top-right buttons, boxes, overlay and chat on the bench

What was learned driving the `[HUD]` panels of `ui/HUD.md` ("The resources, the
buttons, the boxes, the overlay and the chat"), for the agents after.

- **Setup**: `session start --no-launch --res 1280x720`, then `ui/bench-asi.sh
  <state.json> --no-grid HUD.Resources=1 HUD.Buttons=1 HUD.Messages=1 HUD.Pause=1
  HUD.Overlay=1 HUD.Chat=1`, then `drive launch`. `bench-asi.sh` copies a fresh
  `UI.ini` each time, so a key left out is back to 0.
- **The pause key is `Pause`** (`drive key Pause`, Input.map's `pauseGame`); a second
  press unpauses. Pausing and unpausing each post a chat line ("<name> paused the
  game."), the cheapest way to get a chat line in single player: a line typed in
  single player goes nowhere and adds none.
- **Chat opens on Return** in a match; `drive type` then fills the line and the
  engine moves the caret with Left/Right. The prompt is the player's name ("Cedric: ").
- **Objectives**: Instant Action has none (the button is disabled). The Federation
  campaign's first mission opens its briefing box at the start; the Starfleet Academy
  tutorials (single player screen, top-left panel at 300,110; "Tutorial: User
  Interface" at 621,158; O.K. at 555,580) open one per lesson and advance on Return,
  which makes them the test for "do the keys still reach the engine". The first
  Return after a box closes sometimes does nothing; the second advances, stock too.
  The tutorial never ticks its objectives, so the check boxes were seen empty only.
- **Mouse wheel**: `drive step "Scroll the mouse wheel up 4 times"` (there is no
  `drive scroll`, and `click --button 4` is refused).
- **Stock comparison**: with a top-right panel of ours, the stock objectives box did
  not close on Return while the pointer rested on our buttons (no component owns the
  mouse there, so the stock box gets no focus-step). Move the pointer to the 3D view
  before pressing keys at a stock box.
- **Not found on the bench**: a `MessageDisplay` (no mission script reached in a few
  minutes showed one; the tutorial's `*_text_end.txt` is probably one, at the end of
  a tutorial), a replay (the `ReplayPanel`), multiplayer's comm panel and drop box.
- **The trade status needs the campaign's script**: `-nointro a2_fed02` loads the map
  without it (and deals a random faction), so nothing shows. Go through the menus:
  at 1680x720 with the UI's menus, Single Player 198,258, the Federation campaign
  260,322, `Click "Lifelines"`, `Click "O.K."`, `Wait up to 300 seconds for the
  briefing`, `Click "OK"`. "Sent to Team 2" shows at once; Starbase 27 is at 890,172 and
  the *USS Odyssey* at 655,387 (`scenarios/ui-hud-polish.md`). Every Federation
  mission is unlocked on the bench's profile. The bar's grid keys there: Q orders,
  W formations, E the AI menu, G back.
