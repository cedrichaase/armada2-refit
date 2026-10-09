# Changelog — gui-installer

The graphical installer, `armada2-refit-installer.py`. Its version is `INSTALLER_VERSION`
in the script, which `publish/package.sh` checks against the newest heading here. The
package schemas it reads are listed in each entry. Newest first.

## 1.3.0 — 2026-10-09

Reads package schemas 0 and 1.

### Added
- A procedural nebula behind the stars (`nebula_pixels`), new at every launch, faded in
  and drifting slowly; it replaces the three fixed colour washes.
- Warp while installing (`Sky.set_warp`): the stars streak and everything speeds up,
  easing in and out, held for at least 3 s.
- An amber accent (`AMBER`): the Install button, the progress bar and libadwaita's accent.
- When no Heroic is found, an open row with how to install it on this distribution
  (`HEROIC_HOWTO`, `heroic_installed()`), a link to its site and what to do next.

### Changed
- Heroic's launch settings are always written when Heroic has the game: the switch is
  now a line saying what goes in. The launch variables to copy show only for a game
  Heroic does not run. `--no-launcher` stays on the command line.

### Removed
- The "Remastering tools for Star Trek: Armada II" subtitle and the note that textures
  are not shipped.

Installed, not yet seen in game.

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

Installed, not yet seen in game.

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

Installed, not yet seen in game.

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

Installed, not yet seen in game.
