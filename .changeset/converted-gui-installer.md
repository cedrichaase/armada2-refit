---
layers: [gui-installer]
bump: minor
status: Confirmed in game 2026-11-06
---

Reads package schemas 0 and 1, unchanged.

### Added
- **Launch game** shows that the request arrived (issue #48): at once the button is off and reads "Launching…" with a small spinner (GTK) or a turning amber arc beside the label (Tk), then "Game running" until the game exits, when it comes back. A second click or key repeat is refused in the handler as well (`LaunchWatch.begin`). It counts as launched when the game's process is seen (`/proc` on Linux for Heroic, `tasklist` on Windows, `ps` on macOS for the prefix's Wine or a wrapper) and at least `LAUNCH_MIN` (4 s) have passed; a process never seen within `LAUNCH_GIVE_UP` (60 s) re-enables the button and says so in the status line, as does a launcher that fails. The process list is read on a thread, every 1.5 s.
- `--selftest`: the launch state machine.
