---
ui: patch
testbench: patch
status: Installed, not yet seen in game
---

### Fixed
- The engine's text is read as Windows-1252, its code page, where it was read as Latin-1 (`text::game`, at the tooltips' string streams and `engine::buttons::clean`): a bullet, dash or curly quote no longer draws as `?`; a tab draws as a space and other control characters as nothing; the font bakes the rest of Windows-1252.
- Fleet Operations: Roots: with `[HUD] Tooltips=1` the tooltips no longer show symbols and question marks. Roots' fonts draw some codes as red figures, resource icons and unit-class words; the content set's new `[Glyphs]` table (`ui/content/sets/fleetops/ui.ini`) names each, and the UI writes the digit or word in its place. The resource strip still names its resources from Roots' long tooltips (decision 153) when the short one is a glyph alone. Scenarios `fo-ui-tooltips` and `fo-ui-tooltips-off` (`a2test --game fo-roots --install .`).
