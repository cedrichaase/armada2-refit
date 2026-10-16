# Operating the Options family of screens (`ui/MENUS.md`, "The Options family")

For agents driving Options, the three settings screens and the confirmations when the UI
hosts the menus (`ui/bench-asi.sh --menus`). Screen places below are 1280x720 and move
with the form's layout (the specs lay out in units, not in the stock design pixels), so
`Click at design X,Y` no longer lands on them: click by text where there is text, or at
these screen places.

- **Options** (from the main menu): Graphics Settings (640,308), Sound Settings
  (640,346), Game Settings (640,383), Previous Menu (534,449). In a mission the nine
  entries stand in one column from y=196, 37.5 px apart (Abort Mission 383, Exit to
  Windows 495) and Return to Game is at (534,561).
- **Graphics Settings**: Display Mode drop-down (440,239): a click opens our list, rows
  from y=268, 27 px apart; the Display Driver (440,294); the switches on the right
  (Hardware Vertex 839,239, Bump Mapping 839,294, Cinematic Window 839,348, Asynchronous
  Cursor 839,403) read "On"/"Off" and step when clicked; Gamma (y=345) and Selection
  Effect Opacity (y=391) sliders across x=255..626; Graphics Detail (y=453) across
  x=653..1025; Previous Menu (328,556).
- **Sound Settings**: the four volumes at y=179, 226, 273, 319 across x=435..845; Sound
  Channels (640,370), Unit Speech (640,424), Hardware Acceleration (640,479); Previous
  Menu (507,585).
- **Game Settings**: sliders at y=195, 241, 288, 335, 388, 448 across x=255..626; Game
  Difficulty (839,198), Interface Mode (839,253); Previous Menu (328,570).
- **A slider takes a click**: `Click at X,Y` on a slider's track sets it to that point (the
  stock bar takes one click and nothing else; our slider shows a drag and sends the
  click on release). The value shown afterwards is the stock bar's own, so it is
  quantised (Gamma has about ten steps).
- **Settings persist in the clone** across a relaunch of the same session
  (`ARMADA.PRF`), and across leaving and opening a screen: the way to check a control
  works exactly as stock is to leave the screen and open it again.
- **The message and the questions** are the same window: the title is the stock label
  ("WARNING:", "SAVE GAME"), the text wraps, the buttons are in a row (Yes 572,417, No
  707,417; O.K. 640,417).
- **Main menu Exit leaves the game at once** (no question): the question is the
  in-mission Exit to Windows, and Abort Mission.
- **A display mode chosen in Graphics Settings is kept** in the clone's options: the list
  showed it again after the screen was reopened and after a relaunch of the session. Choose
  the session's own mode again before leaving, or the next game starts in the other one.
