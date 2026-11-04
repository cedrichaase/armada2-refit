# Operating the Refit screen (`ui/MENUS.md`, "The Refit screen")

For agents driving the Refit screen: a modal of ours over the hosted Options dialog,
in the shell and in a mission. `testbench/scenarios/ui-menus-refit.md` is the worked
example; the contracts (entry, row labels, log lines) are R1 to R5 of the phase 6 plan.
It needs the redrawn menus (`[Menus] Enable=1`, the shipped default); with `Specs=0` or
`Enable=0` there is no `Refit` entry.

- **Reach it**: shell, `Click at 228,390` (Options in the main menu spec), then `Click
  "Refit settings"`; in a mission, `Press Escape`, `Click "Refit settings"` (the tenth entry, directly under
  Game Settings). `Wait for the refit screen` (`ui.json`, text
  "Graphics", "Interface"). UI.log: `refit: opened in the shell (Graphics 19, ...)` /
  `... in a mission (...)`.
- **Everything is clicked by its label**: tabs (`Graphics`, `Interface`, `Gameplay`),
  rows (`Sky`, `Ships`, `Anti-aliasing`, `Theme`, `Right-drag
  pan speed`, `Pay when queued`), the foot (`Reset tab`, `Show advanced`, `Done`). A click
  anywhere on a BOOL row toggles it; a click on an ENUM row's label opens its drop-down,
  and the choice is then clicked by its words (`2x`, `Borg`, `Stock`).
- **Words OCR can read** (exact): the badges `Now`, `Next mission`, `Restart`, `Restart
  needed`, `Must match`, `Locked`; the banner `Restart the game to apply: <labels>`; the
  help strip's sentences `Applies at once.` / `Applies at the next mission.` / `Applies
  when the game is started again.` and `Every player in a network game must have the
  same.`; the Graphics foot `Drawn on Vulkan`.
- **What each change leaves** (grep these, not the screen): UI.log `refit: [<Stem>.<Section>]
  <key> <old> -> <new> (now|next mission|restart)`, then the hub's `settings: RefitPlayer.ini
  [S] K=V written` and the plugin's `settings: applied ...` (Render3D.log: `settings:
  applied Modules.sky=0`; QOL.log: `settings: applied QOL.PanSpeed=1 (coefficient ...`).
  `Expect "RefitPlayer.ini" contains "sky=0"` reads the overlay file itself (it is a log
  beside `Armada2.exe`, kept by `collect_logs`). A row set back to its default leaves the
  overlay: `Expect "RefitPlayer.ini" not contain "sky="`.
- **Esc closes only the modal**: UI.log `refit: closed`, and in a mission there must be no
  `menus: Esc -> Return to Game`; Options (or the in-mission menu) is still up, so
  `Expect "Return to Game" is visible` holds. A second Esc then behaves as before.
- **The overlay survives `Launch the game` twice in one scenario**: `empty_overlay` runs
  when a session is created, never in `launch`, and `collect_logs` keeps
  `RefitPlayer.ini` (`overlay_selftest.py`). The logs are deleted by each launch, so
  lines from the first game are gone after the second. `Quit the game` before the second
  launch.
- **Locks are for a network mission only**: `Locked` shows on a `Must match` row when the
  menus are over a mission and the game is a network game (or its mode could not be read).
  The shell and a single-player mission show `Must match` alone. The multiplayer lock is
  checked in `multiplayer-two-players` (two games): `Click "Pay when queued"`, then UI.log
  `refit: [QOLRules.Rules] PayOnQueue locked in a network game`.
- **Traps**: `Graphics` also reads inside "Graphics Settings" (Options), so wait for the
  screen, not the word; `Stock` appears in a drop-down only after `Click "Right-drag pan
  speed"`; `Show advanced` is session-only and starts off, so `Largest selection` is not
  on the screen until it is clicked. A tab with no visible row is left out, so the tab
  counts in `opened` differ with the plugins installed. A plugin that stood down shows
  `refit: Render3D: <note>` in UI.log.
- **Passthrough check**: with `Ships` off, the engine draws the ships (stock look, no
  per-pixel lights, no bloom); that is a judged step, since a pixel baseline cannot say it.

## Screen places at 1280x720

The modal is 800x547, centred. Tab text and the title read in upper case. Tabs at y=138:
Graphics x=323, Interface 470, Gameplay 621 (three tabs since 15.6.12). Foot at y=611:
Reset tab 315, Show advanced 449 (box 399), Done 965. Rows from y=169, 36 px apart; left
column labels at x=261, right at x=661; switches at x=575 / 975, drop-downs at x=538 / 938.
Graphics: Anti-aliasing 169 (box at 538,176), HDR and bloom 205, Ships 241, Sky 277.
Interface (right column): Theme at 938,356. Gameplay: Right-drag pan speed at 938,176.
An open drop-down's list starts 29 px under its box and its rows are 26.7 px apart (Theme:
Auto 871,385; Borg 871,545; pan speed: Stock 868,205, 2.5x 868,285). Options' `Refit` entry
is at (641,407) in the shell and (641,538) in a mission.

**Click rows and drop-downs by place, not by label.** The help strip repeats the words of
the row under the pointer (`auto takes your race...`, `Sky`, `stock`), so OCR finds the label
there first and the click lands on the strip. Tabs, `Done` and drop-down choices that the
help strip does not repeat (`2x`, `4x`) are fine by label.
