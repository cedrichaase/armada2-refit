# Playing a match: notes for bench agents

`judge.agent()` puts this file into the prompt of every agent step, so an agent that
has to *play* (build, mine, fight) starts from what earlier runs learnt rather than
clicking around. Everything here was observed in the running game through the bench.
Pixel positions are from **1920x1080 (16:9), refit**, and move with the resolution;
the layout they describe does not. When a note turns out wrong, fix it here.

Agents report what they learnt in their result (`lessons`); the bench writes those into
the case log and appends them to `agent-lessons.md` in the results directory. Fold the
ones that hold up into this file.

## Getting a match with units

Use `Include "_enter-instant-action"` (in `scenarios/`): Instant Action on Warzone, as
the Borg, against one easy AI. Each of these cost a bench round on 2026-10-04:

- **A direct map launch has no units.** `Launch: -nointro a2_borg01` (or `--args` on
  `session start`) opens the map with an empty fog-grey view and an empty bar. It is
  fine for HUD looks, useless for anything that needs a unit.
- **Instant Action will not start alone.** LAUNCH with every other slot empty answers
  "Launch Failed: The game cannot be launched with only one player." Slot 2's name
  dropdown (design 124,115) offers Empty Slot, Computer AI:Easy (design 124,143),
  Medium and Hard.
- **Pick the side.** Side defaults to Random, which deals any race, and the units and
  panels differ per race. The Side dropdown (design 246,95) opens over the rows below
  it; Borg is design 246,123.
- **The match is up about 5 seconds after LAUNCH.** Waiting 50 wastes most of a minute
  per relaunch; the fragment waits 8.
- **Where things are** (1920x1080, Borg, Warzone, the same every time): the Nexus at
  1130,470, an Assembler at 960,388, another just right of it, a Resource Collector
  below at about 1062,713. Tooltips confirm ("G204-… - Assembler").

## The screen in a match

- The view opens top-down on your own start: the starbase and a few ships.
- **Minimap** bottom left. Clicking it centres the main view on that point: the fastest
  way to move the camera.
- **Info panel** bottom centre: the selected unit's class name (magenta), its ID and
  stats. **Unit view** bottom right: a 3D view of the selection.
- **Counters** top left: dilithium, metal, crew, and a count/limit (`16/600`) that went
  up by one when a ship was queued.
- **Hovering** a unit, a resource or an icon shows a tooltip after about a second:
  `G204-45605 - Assembler`, `Dilithium Moon`. `drive ocr` reads it. Use it to find out
  what something is before clicking it.

## Units, by faction

Class names differ per faction; read them off the tooltip. Seen for the **Borg**:

| Role | Borg |
|---|---|
| starbase | Nexus |
| construction ship | Assembler |
| scout | Detector |
| mining station | Resource Processing Node |
| freighter | Resource Collector |

## Commands and building

- **Selecting** a unit (left click on it) shows its **action bar**: square icons in a row
  just above the info panel (y≈778, from x≈600, about 75 px apart). With
  `GridLayout.asi` installed (grid layer, on in refit) the bar is instead a 5x3 grid of
  labelled buttons, and at 16:9 and wider it sits at the bottom between the minimap and
  the info panel, which moves right. Its keys are by position (QWERT / ASDFG / ZXCVB;
  T cancel, G back); `GridLayout.log` lists what each menu put on which key, which is
  quicker than reading icons.
- **Stock bar keys work without the menu open**: with only an Assembler's top level
  showing, F5 starts placing a Processing Node. B opens the build menu, C orders, N AI,
  F formations. With `GridLayout.asi` these are all off, and Q opens the build menu.
- For the Assembler the **4th** icon (x≈823) opens the **build menu**. It replaces the bar
  with structure icons; dark ones are not available yet. The arrow in the row above goes
  back.
- **Read an icon before clicking it**: `drive move` onto it, `drive wait 2`, then
  `drive ocr`. The tooltip gives the name, hotkey, what it does and the cost. In this
  run the Resource Processing Node was the 5th build icon (x≈897); confirm it by tooltip,
  since the order is the faction's.
- **Placing a structure**: after choosing it, its footprint follows the cursor. **Red
  brackets mean it cannot go there.** Move until they are not red, then left-click.
  Clicking on red prints "Cannot build at specified location." top left and **ends
  placement**: open the build menu and choose the structure again.
- The construction ship then flies there and builds. The Node took about two and a half
  minutes. Check every 30-60 s (`drive wait 30`, then one shot), not every few seconds.
- A **starbase** builds ships the same way: select it, and its bar shows the ships it can
  build (the Nexus: Assembler 1st, Resource Collector 2nd). A click queues one.

## Resources

- A **Dilithium Moon** is a *small* blue-grey sphere, about 40 px across at the opening
  zoom. The big planets are not it. Hover to be sure.
- A mining station goes **beside** the resource, not on it. A Node placed 70 px right of
  the moon's centre was refused; 120 px right was accepted.
- A freighter mining shows as a blue beam between it and the moon; then it flies to the
  station. A start can already include a freighter.
- With **Resources: Unlimited** (GAME SETUP) the counters stay at 999999, so a delivery
  cannot be confirmed from the numbers; watch the freighter instead.

## Traps

- **Escape opens OPTIONS**; it does not cancel anything. In multiplayer the match keeps
  running behind it. Leave with **Return to Game**, bottom left (≈403,1042).
- **Right-click cancels a placement** (the footprint goes, the bar comes back).
- **A hung game looks like a black screen with a frozen game cursor.** The game draws
  its own cursor, so if `drive move` moves the host pointer and the game's cursor stays
  put, the game is not running frames any more, even though `Expect the game is still
  running` passes. Don't keep clicking: `Stop the game`, then read the plugins' logs
  (the one time this happened, a plugin under test had crashed the update loop).
- A screenshot after every hover is what made the first mining run take 108 turns. Batch
  move, wait and ocr, and take a shot only to decide something.
- **Driving a session from tool calls**: each command starts a fresh shell, so an
  `export A2TEST_SESSION=…` does not carry over. Put the session path and a list of
  steps in a small script in the scratchpad and run that.
- **In a scenario file, every numbered or bulleted line is a step**, including the ones
  in a fragment's description. Write notes as prose.
