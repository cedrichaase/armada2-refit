# asiloader — the ASI loader of the games that ship none

`install` puts the Ultimate ASI Loader (4.68, x86; ThirteenAG, MIT, vendored unmodified in
`platform/vendor/STA2WidescreenPatch-1.0/winmm.dll`, sha256 in its `SOURCE.txt`) into the
executable's folder as `winmm.dll`, so that the game loads the `.asi` plugins beside it.
`install --remove` takes it out (only if it is byte for byte that file) and puts back a
`winmm.dll` that was there (`winmm.dll.a2asi-backup`). It is not Armada II's: there the
loader belongs to the platform layer, which `a2mod` never switches, and `install` refuses to
run under `A2_PROFILE=armada2`.

For now it is Fleet Operations: Roots' (`games/fo-roots.toml`, code only: not in a release,
the GUI installer or the player's guide). Not part of any `::step` of the release.

## Why, and why only with Display8

Measured in FO-2 (`PLAN-FLEETOPS.md`):

- Roots ships no ASI loader. Its `d3d8.dll` is DxWrapper, which loads `.asi` files only with
  `[Plugins] LoadPlugins=1` in `dxwrapper.ini`; once Display8 takes `d3d8.dll`'s place,
  DxWrapper is not loaded at all and nothing loads our plugins.
- The Ultimate ASI Loader as `Data/winmm.dll` beside Display8 loads `.asi` files normally,
  after `FleetOpsHook` has patched the executable.
- The same loader beside DxWrapper kills Roots at start (`c0000409`), with nothing of ours
  loaded. So the layer `requires` Display8 and goes and comes with it; never install it on
  a clone that still has DxWrapper's `d3d8.dll` in the slot.

Wine must load the game-directory `winmm.dll`: Roots' Heroic entry already has
`winmm=n,b` in its overrides (`games/fo-roots.toml`, `[install]`).

## What installs on Roots now

`./install --game fo-roots`: Display8 (`d3d8.dll`, DxWrapper's kept as
`d3d8.dll.a2display-backup`), this layer, then render3d (`Render3D.asi`, `Render3D.ini` and
the recipes), all into `Data/`. Nothing else is Roots'.

`a2mod --game fo-roots` files `winmm.dll` under this layer only when it is the loader (by
hash); a `winmm.dll` of the player's own is not a change. On Armada II no layer names
`winmm.dll` (`tools/layers.py --check` asserts both).
