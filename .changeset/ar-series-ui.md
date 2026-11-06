---
layers: [ui]
bump: minor
status: Confirmed in game 2026-11-06
---

### Added
- The UI's content set is data (AR-6, root 15.10.0): `ui/content/sets/<set>/ui.ini`, built in (`build.rs`, `src/content_sets.rs`) and parsed by `content.rs` at first use. Besides the races, resources, campaigns and views it held, it names the loading screen's campaign map prefixes and lines (were in `loading.rs`), the race icons (`allrace.tga`), `Credits.txt` and the logos, the cursor sprites (`cursor.spr`) and the widget gallery's lists. The stock set reproduces the old values exactly (`render3d/hosttest`); the log lines are as before.
- `ui/lint-content.sh` (CI): no game file's or stock race's name as a literal in `src/` outside `content.rs`, but for marked engine defaults (`// lint-allow(content):`, the shell's button sounds).
- The engine boundary (AR-4, `PLAN-ARCHITECTURE.md`): `src/engine/api.rs` (`crate::engine::api`), the one facade the rest of the UI reaches the engine through; `hud/`, `menus/`, `app.rs`, `input.rs`, `clock.rs`, `content.rs`, `lib.rs` and the others import it alone. Input's site patches and stock calls moved to `engine/input.rs`, the simulation's clock read to `engine/clock.rs`.
- `trait Panel` (`hud/panels.rs`): name, `[HUD]` key, the claimed component and its rectangle in canvas units, `with_hud`, `modal`, draw; the smaller panels are one table, `PANELS`, in place of the `take(...)` calls. `trait Screen` (`menus/screens/mod.rs`): the dialog's key and draw; `Spec` implements it, `SPECS` keeps its shape and `screens::find` returns the trait.
- The HUD canvas once, `CANVAS` in `engine/canvas.rs` (`api::CANVAS`), read by `hud::panel_rect`, the unit view and the overlay messages.
- `ui/lint-layers.sh`, run in CI: no path into the engine but `engine::api` outside `src/engine/`, no address literal there, the canvas nowhere but `canvas.rs`.

No behaviour change intended: the order of the `panels: ... ours` log lines at a mission's start follows the table (draw order).

### Changed
- The engine resolver (`crates/a2engine`, `csdk/a2engine.h`; root 15.8.0) names the executable in its log lines and notes by the matched build's `exe` (`profiles/*/build.toml`), or on an unknown build by the process's own module file name, not by a literal: `engine: Armada2.exe timestamp ...` on Armada II as before. The generated symbol tables carry each build's `exe`. UI.log recognises the identity line of any executable (`a2engine::is_identity_line`) when it logs it once over its tables.
- The game files the UI reads (textures, `Credits.txt`, `Sprites/cursor.spr`, the loading screen's `bzn`, objectives, sky faces and SOD, the menu plates) go through one `a2fmt::vfs` file system (`ui/src/files.rs`): `textures\` mounted from `Textures\RGB` (`LAYOUT`, held to `games/armada2.toml` `[files]`), the game folder behind it. The same files are read as before.
- `widgets/theme.rs` keys the six race palettes by a look (`Form`: `lcars`, `blade`, `wing`, `arch`, `cube`, `organic`); the set gives each race its look (`Theme =`), and `Race` is a race's place in the set's list. `Theme::all()` is the shell and each race of the set.
- Engine fields and values come from the matched build at run time (`field::X()`, `value::X()`), no longer compile-time consts of `gog-1.1`; the per-system wireframe sprites, the builder's queue, the setup's load fractions and the overlay's system order are sized by the build's counts at run time (`Vec`), and the repair ids are looked up per system (PLAN-ARCHITECTURE.md, AR-3).
- `engine/sprite.rs` reads a sprite's name through the profiled `core_element_name` (its `ELEMENT_NAME` duplicated it), and its texture rectangle and texture through the new `core_sprite_uv` and `core_sprite_texture` of `ui.core.toml`.
- `engine/overlay_text.rs` walks the message map's tree through new `ui.hud.toml` fields (`otext_tree_left`, `otext_tree_parent`, `otext_tree_right`), no longer its own `NODE_*` consts.
