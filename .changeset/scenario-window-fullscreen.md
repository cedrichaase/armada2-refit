---
layers: [testbench]
bump: minor
games: [armada2, fo-roots]
status: none
---

### Added
- Scenario step `put the window "TITLE" full screen`: the session's sway makes that X11 window full screen. `display-window-follow` uses it for the popup (sway puts a later window behind the full-screen game window, where it stays black and its presents are held back to a few a second) and checks both shots are not black.
