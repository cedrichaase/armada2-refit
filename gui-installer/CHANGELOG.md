# Changelog — gui-installer

The graphical installer, `armada2-refit-installer.py`. Its version is `INSTALLER_VERSION`
in the script, which `publish/package.sh` checks against the newest heading here. The
package schemas it reads are listed in each entry. Newest first.

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
