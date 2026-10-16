# Operating the multiplayer screens and boxes with the UI's specs

`ui/bench-asi.sh --dplay --menus` (Microsoft's DirectPlay in the clone, the UI hosting
the menus); add `--online` for *Internet - Online* with a bench server, and
`HUD.Comm=1 HUD.DropPlayer=1 HUD.LoadWait=1` for the multiplayer boxes. Where things
are at **1280x720**, and the traps.

## Where things are

- **Main menu** MULTIPLAYER: 228,346.
- **Multiplayer Connection** (a window in the middle): the name field 640,271, then
  *Internet - GameSpy* 640,317, *Internet - Manual IP* 640,360, *Internet - Online*
  640,403, *Local Area Network (TCP/IP)* 640,445, Previous Menu 488,494. OCR finds the
  buttons by their text too.
- **Manual IP / the online prompt**: the field 640,355, O.K. 659,416, Cancel 784,416.
  The prompt says "Ip address of the host (leave blank to host):", or with *Internet -
  Online* "Join code or host address (blank to host):".
- **The lobby** (`lobby.rs`): Join Game 572,362, Create Game 725,362, the filter
  711,104, the game rows from y ~125 in the current games window, the chat log from
  y 393, Help 987,685, Previous Menu 107,685, Main Menu 1171,685.
- **The game's name** (Create Game): the name field 640,317, the password 640,369, O.K.
  659,430.
- **Game Setup**, multiplayer: rows at y 122, 147, ... 25 px apart; columns: player
  x ~180, side ~370, armada ~510, team ~637; ready lights at x 35; the chat line
  300,640 and its target ("All") 658,641; LAUNCH 987,685.
- **In the match**, the stock top-right comm button is at 1194,16 (the fourth icon).
  The comm panel (ours) hangs from a fixed height (Cardassian theme; the other themes'
  insets move things a few pixels): the players' rows from y 179, 25 px apart, their
  chat switch at x 723; ALL 918,180, ALLIES 918,212, ENEMIES 918,244, PRIVATE 918,276;
  UNITS 918,331; the resource rows at y 363, 393, 424, 455 (minus 896, plus 963, give
  1015); Allied Victory 241,550, High Latency 452,550; DONE at the bottom right (its
  height follows the number of players and resources: click it by its text).

## Traps

- **Typing into a field of ours** works only after a click on it; while it has the
  keyboard the stock edit gets nothing but what the field sends it (before, Multiplayer
  Connection's name was typed twice, the stock edit holding the focus).
- **Enter in the chat line sends it** (the field hands the dialog its Enter with the edit
  focused: Game Setup's procedure reads the line only when its edit has the focus).
- `--dplay` alone leaves `Online.ini`'s `Server=` at the public server: never take
  *Internet - Online* without `--online` (which points it at the bench server).
- **Allied victory does not change in a game whose teams are locked**, in stock too:
  the toggle flips, the order goes out, and the panel puts it back from the team's flag,
  which only a game with free alliances changes.
- **Giving needs an ally and instant trade**: the joiner on the host's team (the team
  drop-down), and the host's Advanced panel's "Trade Using Convoys" clicked once (the
  give buttons are disabled with convoys). Between teams the give does nothing, in stock
  too.
- **The load wait** shows only while the other player is still loading: hold one game's
  load with `--for joiner Loading.Hold=8` (and `Loading.Enable=1`); the drop box then
  alternates with it, so look for its `UI.log` line or take several screenshots.
- **The drop box**: kill or stop the other game; its buttons are disabled for some tens of
  seconds, then Eliminate removes the player.
- **A game alone with a computer is a network game**: Manual IP, Create Game, LAUNCH
  starts a match on the host alone whose comm panel works (two bars, the computer's
  chat switch disabled), which is the cheap way to see the panel with one bench slot.
- The map preview in a multiplayer Game Setup stayed black on the bench (Warzone).
