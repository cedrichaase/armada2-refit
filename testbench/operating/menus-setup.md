# Operating Game Setup and the multiplayer screens through the UI's specs

`ui/bench-asi.sh --menus` (`ui/MENUS.md`, "The setup and multiplayer specs"). Where
things are at **1280x720**, and what is easy to get wrong.

## Where things are

- **Game Setup** (Instant Action: main menu column, 228,302). The Players window is at
  the left; a row's three drop-downs are at x 200 (player), 450 (side), 630 (team),
  rows at y 122, 147, 172, 198, 223, 248, 274, 299. An open drop-down lists its items
  below it about 23 px apart: player 2's (200,147) has Empty Slot 172, Computer
  AI:Easy 195, Medium 217, Hard 240; player 3's (200,172): Hard at 265. Player 1's
  side (450,122): Federation 147, Borg 169, Romulan 192, Klingon 215, Cardassian 237,
  Species 8472 260, Random 283.
- A row's colour swatch is at x 36; the palette's swatches from x 170, 27 px apart, at
  y 345. A click on a row's swatch and then a palette swatch recolours that player.
- The map window is at the right: SELECT MAP 1124,148, then the settings (Minimap
  Hidden, Shroud, Random/Fixed Placement, Show All) down to 276; Resources 855,423,
  Default Settings 1120,423, Save 855,455, Load 1120,455, Advanced 855,487. LAUNCH
  987,685, Load Game 1171,685, Previous Menu 107,685. OCR finds all of them by text
  (`Click "Random Placement"`), which is the better way.
- **Map Select** (SELECT MAP): rows from y 169, 24 px apart, x ~450; the down arrow at
  918,577, the up arrow 918,169; Select Map 420,612, Cancel 858,612. One click on an
  arrow scrolls one row.
- **Advanced**: Weapon Strength's bar at y 156 (x 285..625), Ship Build Time's at
  (655..995, 156); Close 354,605, Default 925,605. Toggles and settings are clicked by
  their labels.

## Traps

- **Do not click a drop-down's item by its text when the same text is in another
  row**: "Empty Slot", "Random" and "Team 1" are in every row. Click the open list by
  place.
- **Instant Action's "Minimap Hidden" does nothing when clicked**, in stock too.
- **Clicking "Default Settings" empties the computer slots.** It is next to Resources.
- The message log under the players says what changed ("Resources: Lots"); it is a
  stock edit, read by the spec, and OCR reads it.
- A dialog opened from another (Map Select, Advanced) is drawn over Game Setup dimmed;
  OCR still reads Game Setup's words through the dimming, so `Expect "X" is not
  visible` is only safe for words that are not on Game Setup.
