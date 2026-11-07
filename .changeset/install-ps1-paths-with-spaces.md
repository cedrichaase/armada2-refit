---
layers: [root]
bump: patch
games: [armada2]
status: Untested: not yet past the Windows gate
---

### Fixed
- Windows release installer (`install.bat` / `install.ps1`): a game directory with spaces works whether quoted with a trailing backslash (`"C:\My Games\Armada II\"`, which failed with "Illegal characters in path") or unquoted (which was cut at the first space); options after it still apply.
