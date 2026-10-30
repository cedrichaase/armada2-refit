# Changelog — gui-installer

The graphical installer, `armada2-refit-installer.py`. Its version is `INSTALLER_VERSION`
in the script, which `publish/package.sh` checks against the newest heading here. The
package schemas it reads are listed in each entry. Newest first.

## 2.3.0 — 2026-10-30

Reads package schemas 0 and 1, unchanged.

### Changed
- The bug report collects the plugins' logs from `%LOCALAPPDATA%\armada2-refit\logs` (on Windows, in the Wine prefix on Linux and macOS: Heroic's `winePrefix`, the bottle, or the prefix the game folder is in), and from the game folder as the fallback for builds before 13.50.0; each log says which. A selftest check covers the prefix folder and the order.

Installed, not yet seen in game. (HASH)

## 2.2.0 — 2026-10-29

Reads package schemas 0 and 1, unchanged.

### Added
- The bug report reads `Display8.log` (it was not collected before) and states in its
  findings the mode, the backend chosen (vulkan, d3d12, d3d9) and why: `Backend=auto`'s
  order, a fallback with its reason, an order set by hand, or a stand-down to forward
  mode (`display8_backend`; selftest checks). All three systems share the code.

Confirmed in game 2026-10-30.

## 2.1.1 — 2026-10-28

Reads package schemas 0 and 1, unchanged.

### Fixed
- macOS: the `display8` layer's line says "will install", as on the other systems, and
  no longer "on Direct3D 9": `install.sh` now leaves `Display8.ini` on `Backend=auto`
  there (root 13.36.1). `test-macos.sh` expects `Backend=auto`. `4c0f9e4`

Confirmed in game 2026-10-30.

## 2.1.0 — 2026-10-25

Reads package schemas 0 and 1, unchanged; a package from 13.35.0 has the layer `display`
(`when` `display8`) and the step `display`, which 2.0.0 shows as "will install" and counts
as it comes.

### Added
- Display8 in the game folder (`is_display8`, by the panic message every build carries):
  `inspect` reads the chain's `d3d8.dll` behind it (`d3d8_real.dll`), so a reinstall
  over Display8 still sees d3d8to9, and Display8, which holds the word "dxvk", is not
  taken for DXVK. The `display8` layer goes in only where d3d8to9 will be, on macOS on
  Direct3D 9; the status line "Installing Display8".
- The bug report names Display8 and `d3d8_real.dll`, and no longer says the plugins get
  no Direct3D 9 device when Display8 stands in front of d3d8to9. `2af094a`

Confirmed in game 2026-10-30.

## 2.0.0 — 2026-10-22

Reads package schemas 0 and 1, unchanged; a package's `bloom` step now takes out the bloom
an earlier one set up.

### Removed
- Everything vkBasalt: the "Bloom needs vkBasalt" panel, `--vkbasalt-howto` and
  `--command`, the per-distribution instructions (`VKBASALT_HOWTO`), and the
  `ENABLE_VKBASALT` / `VKBASALT_CONFIG_FILE` launch variables. An older release's bloom
  layer is no longer listed; its own `install.sh` still decides.

### Changed
- An install takes the vkBasalt variables an earlier install wrote into Heroic back
  out (`OLD_BLOOM_ENV`), where they are still as it left them.
- The Heroic how-to no longer warns that bloom does not work in the Flatpak, nor the
  macOS one that bloom is not available.

Confirmed in game 2026-10-22.

## 1.8.0 — 2026-10-22

Reads package schemas 0 and 1; schema 1's scripts also print `::error <code> <path>`
lines from 12.42.0, which 1.7.0 and older log as ordinary output.

### Added
- **Errors in words** (`explain`): the headline, what to do and the details, the same in
  the GTK and Tk windows and on the command line. A failure is known by a code, from the
  scripts' last `::error` line (the files of every line of that code named), from the
  installer's own `InstallError`, or from the exception (a bad zip, a network error, a
  file refused or held: `PermissionError`, Windows errors 5 and 32). A script with no
  `::error` line (schema 0) fails with its last line, as before.
- The error dialog: the headline, the steps, a link to the README's "Antivirus false
  positives" where that is the likely cause, the log folded away, and **Copy report**,
  the diagnostics with the failure in them.
- `diagnose`: a finding for files of a release missing or unreadable in the
  installer's cache or the game folder, "likely an antivirus", with the files listed.

### Fixed
- `Package.unpack` no longer wedges on an earlier unpack an antivirus still holds a file
  of: it retries the removal with a short backoff, then unpacks into `<version>-1`,
  `-2`..., cleared at the next unpack (`fresh_dir`, `sweep_unpacked`). A file taken
  or held while extracting is an antivirus error, not a traceback.
- `diagnose` no longer fails on a plugin it cannot read; it is listed as unreadable.
- `diagnose` also checks an unpack against its own `SHA256SUMS`, so a file missing from
  the zip itself is found too.
- The report's redaction keeps a user folder's own separator (`/home/<user>/`, not
  `/home\<user>/`; since 1.7.0).

Confirmed in game 2026-10-22 (the error dialog and Copy report, on Linux).

## 1.7.0 — 2026-10-20

Reads package schemas 0 and 1.

### Added
- **Diagnostics for a bug report** (the menu on Linux, **Report** on Windows and macOS)
  and `--report [--game DIR]`: a plain-text report to paste into an issue or save
  (`diagnose`). It holds the system and launcher (Heroic's Wine, `autoInstallDxvk`, the
  launch variables; a Mac prefix's DLL overrides), `d3d8.dll`, `d3d9.dll`, `winmm.dll`
  and `binkw32.dll` named by hash, every `.asi` checked against the cached release, the
  `.ini` keys changed from it, the plates in `Menus/`, an excerpt of each plugin's log,
  and findings: no ASI loader, UI.asi or Render3D.asi without crosire's d3d8to9, GOG's
  `d3d8.dll` back, old and new plugins together, `Enable=0`, the menu scene off, a
  panic. The home folder, user names and IPv4 addresses are left out (`redact`).

Confirmed in game 2026-10-30.

## 1.6.0 — 2026-10-17

Reads package schemas 0 and 1.

### Added
- Steps `render3d` ("Installing the 3D view") and `ui` ("Installing the HUD and the
  menus"); `hud`, `menus` and `grid` stay for older packages.
- Layer condition `when: d3d8to9`: the 3D view is skipped, and the HUD and menus are the
  older plugins, where the chain step will not leave d3d8to9 as `d3d8.dll`.

### Changed
- An install is recognised by `UI.asi` as well as `HUD.asi`.

Confirmed in game 2026-10-30.

## 1.5.0 — 2026-10-12

Reads package schemas 0 and 1; on Windows and macOS, only packages whose manifest names
the system (11.20.0 on).

### Added
- Windows and macOS. The core runs on all three (`SYSTEM`): per-system cache and settings
  folders, processes, and the release script from the manifest's `windows` / `macos`
  entry (`Package.command`, `Package.for_system`): `install.ps1` through PowerShell on
  Windows, `install.sh` through `/bin/bash` on macOS.
- Finding the game on Windows (GOG's registry entry, `gog_registry_dirs`; the usual
  folders on every drive) and on macOS, in a Wine prefix (`mac_prefixes`): CrossOver
  bottles, Whisky bottles, Sikarugir / Kegworks / Wineskin / Porting Kit wrappers, plain
  prefixes.
- On macOS the DLL overrides go into the prefix's registry, under
  `AppDefaults\Armada2.exe\DllOverrides` in `user.reg` (`prefix_plan`, `prefix_apply`),
  only while no Wine runs, with `user.reg.a2refit-backup`; uninstall takes out what is
  still as it left it. Windows needs none.
- **Launch game** without Heroic (`launch_game_native`): the exe on Windows, the
  prefix's Wine on macOS (CrossOver's `wine --bottle`, a wrapper's own, Gcenx's).
- The Tk window for Windows and macOS (`run_tk`), the GTK window's sky, nebula, warp and
  engine bar drawn on one canvas, and how to get the game (`WINDOWS_HOWTO`) or a Wine
  for it (`MAC_HOWTO`: CrossOver 26; Sikarugir or Gcenx's Wine) when there is none.
  `A2_INSTALLER_TK=1` opens it on Linux.
- The frozen program stands in for Python (`run_python`): the release scripts run the
  SOD rewrites with it (`A2_PYTHON`), so Windows and macOS need no Python.
- `pyinstaller/build.py` (PyInstaller 6.22.3, `pyinstaller/requirements.txt`) and
  `pyinstaller/icon.py`: `armada2-refit-installer-windows.exe` (one portable windowed
  file, icon, version resource) and `armada2-refit-installer-macos.zip` (a universal2
  `.app`, ad-hoc signed). Both are attached to every release beside the AppImage.
- `test-windows.ps1` and `test-macos.sh`: the frozen installers against a mock game (on
  macOS in a mock CrossOver bottle); CI jobs `windows-gui` and `macos-gui`.

### Changed
- The layer lines say what the install will leave, not what is there now: the chain
  step's d3d8to9 counts for lighting and the sky, DXVK's d3d9 for MSAA (`after_d3d8to9`,
  `after_dxvk`). MSAA, `dxvk.conf` and bloom say why they are not on macOS; bloom on
  Windows is ReShade's.
- `STEPS` and the status lines know `chain`, `grid` and `models`; "Writing the launch
  settings" in place of Heroic's alone.

Installs nothing into the game itself. Installed, not yet seen in game.

## 1.4.1 — 2026-10-09

Reads package schemas 0 and 1.

### Fixed
- The AppImage leaves fontconfig and freetype to the host (`EXCLUDE` in
  `appimage/build.py`): the bundled, older fontconfig printed about ninety warnings
  about the host's font configuration at every start.

Confirmed in game 2026-10-09.

## 1.4.0 — 2026-10-09

Reads package schemas 0 and 1.

### Added
- A procedural nebula behind the stars (`nebula_pixels`), new at every launch, faded in
  and drifting slowly; it replaces the three fixed colour washes.
- Warp while installing (`Sky.set_warp`): the stars streak and everything speeds up,
  easing in and out, held for at least 3 s.
- An amber accent (`AMBER`): the Install button, the progress bar and libadwaita's accent.
- A bolder title: "ARMADA II" at weight 900 over a tracked amber "REFIT"
  (`.title-main`, `.title-tag`; `.title-big` is gone), in the desktop's own font.
- **Launch game** beside Install (`launch_game()`, `heroic_launch_uri()`): starts the
  game through Heroic; it takes the amber once there is nothing new to install.
- `EngineBar`: the progress bar glides, with a glowing engine and a trail at its tip.
- When no Heroic is found, an open row with how to install it on this distribution
  (`HEROIC_HOWTO`, `heroic_installed()`), a link to its site and what to do next.

### Changed
- Heroic's launch settings are always written when Heroic has the game: the switch is
  now a line saying what goes in. The launch variables to copy show only for a game
  Heroic does not run. `--no-launcher` stays on the command line.

### Removed
- The "Remastering tools for Star Trek: Armada II" subtitle and the note that textures
  are not shipped.

Confirmed in game 2026-10-09.

## 1.3.0 — 2026-10-09

Reads package schemas 0 and 1.

### Added
- The sky layer (`Sky.asi` and its recipes): its own progress line, and "stock sky without
  d3d8to9" in the layer list when the chain cannot give it shaders.

Not yet seen in a window; `gui-installer/test.sh` runs it against a package.

## 1.2.0 — 2026-10-09

Reads package schemas 0 and 1.

### Added
- `appimage/`: `build.sh` and `build.py` build `armada2-refit-installer-x86_64.AppImage`,
  with Python, GTK 4 and libadwaita inside; `AppRun` keeps the host's environment in
  `A2_ORIG_*`. Built and tested in CI on ubuntu-24.04.
- `host_env()`: the programs the installer starts get the host's environment, not the
  AppImage's. `ssl_context()`: falls back to the usual CA bundles.
- `test.sh` takes `A2_INSTALLER_APPIMAGE=<file>` to run its checks through the AppImage.

### Changed
- `INSTALLER_ASSET` is the AppImage, so the "newer installer" banner links to it.

Confirmed in game 2026-10-30.

## 1.1.0 — 2026-10-09

Reads package schemas 0 and 1.

### Changed
- The window is rebuilt: one screen in plain libadwaita on a starfield drawn in code
  (nothing to ship; still when the desktop has animations off). It holds the game, the
  version, the Heroic switch, a heads-up line per layer that will be skipped, one
  Install button, and the progress. The LCARS styling, the four pages (`Alt+1..4`), the
  layer table, the release notes and the Antonio font download are gone.
- The log moved into a dialog (`Ctrl+L`); `F5` refreshes, `Ctrl+Enter` installs.
- Bloom's "how to get vkBasalt" is a collapsed row with a Copy button per command.
- Status lines are plain ("Installing the HUD"); the command line prints the same.

### Removed
- `FONT_URL`/`FONT_SHA` and the cached font; the window no longer fetches anything but
  the release list and the zip.

Confirmed in game 2026-10-30.

## 1.0.0 — 2026-10-08

Reads package schemas 0 and 1.

### Added
- `armada2-refit-installer.py`: a GTK 4 / libadwaita window in an LCARS style that finds
  the game (Heroic, Lutris, Bottles, Wine prefixes, `A2_GAME`, Browse), lists the GitHub
  releases (checked at launch and on Refresh, cached in `~/.cache/armada2-refit/installer`),
  shows what the chosen release installs on this machine, and installs or uninstalls it
  through the release's own `install.sh`, with a progress bar per stage.
- Writes the launch variables into Heroic's settings for the game (switchable; a backup
  as `.a2refit-backup`), and takes them out again on uninstall.
- Leaves out textures and the cutscene player (`A2_SKIP=cutscenes`); for schema-0
  packages it takes the cutscene player out after their `install.sh`.
- When no vkBasalt layer is found, how to get one on this distribution, detected from
  `/etc/os-release` (Arch, Fedora, Fedora Atomic, Debian, Ubuntu, others), with Copy;
  `--vkbasalt-howto [FAMILY|all] [--command N]` prints the same.
- Command line: `--list`, `--install [VERSION]`, `--package ZIP`, `--uninstall`,
  `--selftest`, `--check-version`.

Confirmed in game 2026-10-30.
