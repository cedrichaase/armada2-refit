---
root: minor
ui: minor
testbench: minor
status: Installed, not yet seen in game
---

### Added
- The UI's mission frame on Fleet Operations: Roots (FO-6 step 3): `core.mission` is verified beside FleetOpsHook's trampoline of `ChatDisplay::Render` and its edit of `Armada_RenderFrame` (`profiles/fo-roots-5.0Z11/build.toml`, `tools/engine/fo-scan.toml` `[verified]`)
- Hook `core.input_callee`: input taken at the entries of `ProcessKeyboardMessages` and `ProcessMouseMessages` for a build whose window procedure is not the engine's `WindowProc` (profile values `core_input_at_callee`, 0 on stock and 1 on Roots, and `core_input_moved`); stock keeps `WindowProc`'s calls
- `[UI] RefitKey`: with Ctrl, opens the Refit screen as a modal over a running mission (0, none, in the shipped `UI.ini`; 123, Ctrl+F12, in Roots' defaults); the key is kept from the engine
- UI.log names the render lists once per mission (`hud: render lists`), the thread's windows and their window procedures at the first loading and mission frames and the first device releases, the callers of the two message functions at their entries, and the first button downs kept from the engine
- The UI's file system follows the game (`ui/src/files.rs`, layout `fo-roots`: Roots' `Textures\`), and a map's sky face is read as TGA or DDS (`a2fmt::image`): the loading screen finds Roots' skies
- Scenarios `fo-ui-mission`, `fo-ui-refit`, `fo-ui-standdown-ref` and `fo-ui-standdown` (`a2test --game fo-roots --install .`)

### Changed
- Whether the engine hides its HUD is read from the info panel the render lists name (`hud::ship_display`), not the `shipDisplay` global; hook `core.reads_hud_hidden` and its symbol are gone from the profile
