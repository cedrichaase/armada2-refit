---
root: minor
render3d: minor
games: [fo-roots]
status: Installed, not yet seen in game
---

### Added
- render3d's `fleetops` content set (FO-5 step 6), named for all of render3d by `profiles/fo-roots-5.0Z11/content.toml` (until now every module but the hulls read the stock set on Roots, inherited from gog-1.1): Roots' names with the stock set's looks. `ordnance.ini` `[Names]` says which ordnance ODF each built-in recipe draws (`Ordnance.<odf> = <recipe>`, `Wave.<odf>`; with it a built-in recipe draws only the ODFs named: Roots' construction ships' tractor beam, the Borg holding beams, Clairvoyance, the Cardassian Energy Drain, the holo-generator, shield inverter and computer override, the miners' dilithium and tritanium beams, the repair beam, two waves; Roots' `ionstm`, another weapon than stock's, no longer takes stock's recipe). `effects.ini` `[Races]` keys the shields' faction by an ODF's `race`. `world.ini` gives Roots' nebula classes the stock recipe of the same `nebulaType` and hue and names no surface and not `latinum` (Roots' asteroid fields). `particles.ini`, `overlays.ini`, `sky.ini`, `specials.ini`: the names Roots keeps, or none
- Fleet Operations' mods in render3d's file system: the active mod from `-mod`/`/mod` on the command line, its `ParentMod` chain from each `info.ini`, every folder of the chain mounted a level above the one under it, `Data\` last unless the chain is `standalone=1` (`render3d/core/src/files/mods.rs`; `games/fo-roots.toml` `[files] mods`, held to the code by `tools/layers.py --check`); `data\` is the game directory itself; a name is found in the top folder that has it, wherever it sits; an ODF's include of its own name is its next version down (`a2fmt::odf` `Odf::load_layered`, `a2fmt::vfs` `Layered::mount_level`, `sources`, `open_nth`); `r3d_core::assets::race`

### Changed
- `RTS_CFG.h` is read through the file system (`data\RTS_CFG.h`): on Roots the active mod's copy over its parents' over `Data\`'s, the one the game reads; stock unchanged (the game directory's, read only where the profile says)
- `render3d/core/src/files.rs` is `render3d/core/src/files/mod.rs`
