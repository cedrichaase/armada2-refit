# retire/ — taking out what earlier versions installed

Not a layer: versioned by the root. `retire.sh <game dir>` takes out of a game directory
everything an earlier version of this project put in and nothing installs any more, by the
table in [`known.txt`](known.txt). `./install` runs it before the first layer installs;
`render3d/install.sh` and `ui/bench-asi.sh` run it too; the release installers keep their
own copy of the table (`package.sh` bakes `known.txt` into the zip as `retire.txt`), with
`hud-files.py` beside them. `a2mod refit` does not run it: it restores an old snapshot
byte for byte and then says which retired layers the snapshot holds.

    retire/retire.sh [--check] [--chain] [<game dir>]
    retire/selftest.sh              # a mock game directory with every entry; host only

## What it recognises

| Entry | How | Action |
|---|---|---|
| `HUD`, `GridLayout`, `Menus`, `MenuScale` (`.asi`, `.ini`, `.log`; `MenuScale/*.bmp`) | by name | removed. The plates in `Menus/` stay: `UI.asi` draws them |
| the twelve per-part plugins render3d replaced, and `Sky/mbg*.ini` | plugins by name; recipes by hash (`glob-hash`: every version ever shipped) | removed; a `Sky/` recipe the player edited, or made, stays, and so does the folder |
| `A2EngineTest.ini` | by name | removed (the bench's fault injection, never shipped) |
| the per-user vkBasalt bloom (`a2-vkbasalt` under `$XDG_DATA_HOME`) | by path; the library and cache only with the layer's manifest | removed |
| `misc/gui_*.cfg`, the cursors `Sprites/cursor.spr` names | their `.a2neb-backup` | restored, then the backup dropped |
| `Sprites/FontFinal4_*.spr`, `Textures/RGB/FontFinal4_*` | their `.a2font-backup` | restored, then the backup dropped |
| `SOD/logo.SOD`, `Textures/RGB/LOADING1..6` (the widened loading screen) | their `.a2neb-backup` | restored, then the backup dropped |
| `d3d8_real.dll`, `d3d9.dll`, `dxvk.conf` | the vendored build's sha256 / our marker line | **chain class**: only with `--chain` |

The chain class is the vendored d3d8to9/DXVK chain Display8 stood on until 15.6.0
(platform 4.0.0), which `platform/d3d8-chain.py --retire` also takes out; a `d3d9.dll` or
`dxvk.conf` the player made is never touched either way.
The samplers of a marked `dxvk.conf` are carried into `Display8.ini` by `tools/ini-merge.py`
before it goes.

## Why the font atlases have a suffix of their own

`hud/ui-font-condense.py` (hud 2.x) rewrote the twelve `FontFinal4_*` atlases in place,
paired with the `.spr` metrics beside them, and kept `.a2font-backup` rather than
`.a2neb-backup` so `a2tex revert all` could never put a stock atlas under condensed
metrics. The same holds here: `hud-files.py` restores atlas and metrics together, from the
same suffix, and drops the backups only once the files are stock again. `a2mod` still lists
`.a2font-backup` under the retired `hud` row so an old snapshot is recognised.

## history/

`history/{hud,grid,menus}-CHANGELOG.md` are the changelogs of the three layers whose
folders were deleted, each ending in its final "Removed" entry. What their READMEs
established lives beside the code that still uses it (`ui/HUD.md`, `ui/MENUS.md`,
`display/README.md`).
