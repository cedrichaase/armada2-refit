# The UI's minimap and selection overlay on the bench

What was learned driving `[HUD] Minimap=1`, `Selection=1` and `UnitView=1`
(`ui/HUD.md`), for the agents after.

- **Instant Action deals a start position at random.** On Warzone the Borg base was at
  the minimap's left middle in one launch and its top right in the next, so a click
  on the minimap at fixed coordinates lands somewhere different relative to the base
  each time. The Assembler at 640,262 is reliable (the camera starts on the base).
- **A minimap order is evidenced in `UI.log`**: "minimap: order 0x2b at x,z" (0x2b is
  the move to a point; "(refused)" if the bar's `BroadcastCommand` said no). The first
  "minimap: ours at X,Y SIZEpx; map texture ...; fog grid ...; N blips" line is written
  once there are blips: its square's place and size is where to click.
- **Right clicks**: `drive click X Y --button 3`, or a scenario's "Right-click at X,Y."
- **Damage on demand**: the `fog` scene (`ui/bench-asi.sh ... --scene fog`, launched
  with `-nointro a2_borg01`) has three of the player's ships, not healed; `scene
  "spawn bad kbattle 2600 0 2600 0 2"`, then `scene "attack bad ship1"` brings
  `ship1`'s hull down within 15 s. `scene "select ship1 ship2 scout"` and Ctrl+1 make
  group 1.
- **The scenes' teams have no map colour** (`Team+0x14` is black without a game
  setup), so stock's minimap dots are black there; ours colours them by relation.
- **The unit view (`CinematicView`) stayed hidden** in Instant Action and in the scene,
  through the top-right buttons (Toggle View switches strategic and tactical; the
  arrow opens Toggle Palette, Toggle Help and the Options Menu). It is shown in
  `DisplayInterface::SetInterfaceState` state 0 only; how a player gets there was not
  found, so `UnitView=1` has not been seen drawing.
- **The tactical view** (Toggle View, the eye at about 1168,16 at 1280x720, or Caps
  Lock: `Press Caps_Lock`) keeps the selection overlay working; the minimap then shows
  the tactical camera's ring, and a fight's shot-director ring (`ui-leftovers-2`).
- **Damaged systems on demand**: in `fog`, `weapons ship1 off` (and the others) before
  the Klingon attacks, or ship1's friends kill it first; after 40-60 s ship1 (Defiant
  class, at about 640,382 after `center ship1` at 1280x720) has systems out. The scene
  hides the cursor (`Cursor=0`): `scene "cursor on"` for a cursor step.
- **Quitting** (`drive quit`) ended with "the game did not exit through its menus; the
  prefix was stopped instead" every time that night (see `hud-takeover.md`); the
  session survives it and `drive launch` starts the game again.
