---
layers: [ui]
bump: minor
games: [armada2]
status: Untested: macOS not yet checked
---

### Fixed
- The system cursor is hidden over the game window whenever the UI draws its own, in every context (the WM_SETCURSOR answer, the class cursor and ShowCursor's count), and given back when frames stop or another program has the foreground; for Wine on macOS, which showed the arrow beside ours. New key [UI] HideSystemCursor (default 1).
