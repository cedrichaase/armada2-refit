# gui-installer — the graphical installer for Linux

`armada2-refit-installer.py` is one Python file: a GTK 4 / libadwaita window in an
LCARS style that installs a **release** of Armada II Refit into the GOG game under
Wine/Proton. It is for players, not for working on the project. The repository's own
`./install` builds from source and installs the asset layers too; this installs what a
release zip carries.

    python3 armada2-refit-installer.py            # the window
    python3 armada2-refit-installer.py --list     # games and releases it finds
    python3 armada2-refit-installer.py --install [VERSION] [--game DIR] [--no-launcher]
    python3 armada2-refit-installer.py --package ZIP --game DIR
    python3 armada2-refit-installer.py --uninstall [--game DIR]
    python3 armada2-refit-installer.py --selftest

The command line runs the same `Job` as the window and needs no GTK, which is how
`test.sh` and CI exercise it. Needs: Python 3 and its standard library; for the window,
PyGObject with GTK 4 and libadwaita (Arch `python-gobject gtk4 libadwaita`;
Debian/Ubuntu `python3-gi gir1.2-gtk-4.0 gir1.2-adw-1`; Fedora `python3-gobject gtk4
libadwaita`). Without them it says so and points at the command line.

Keys: F5 refresh, Ctrl+Enter install, Alt+1..4 the pages.

## What it does

1. **Finds the game**, best first, without duplicates: Heroic's GOG list
   (`gog_store/installed.json`, native and Flatpak; GOG app `1174788223`), `A2_GAME` as
   `a2env` resolves it, Lutris' database, Wine prefixes (`~/.wine`, `~/Games/*`, Heroic's,
   Bottles') and `~/Games`, `~/GOG Games`. The walk only descends into folders whose
   names could lead to the game, so it stays quick. **Browse** takes any folder holding
   `Armada2.exe`; the choice is remembered in `~/.config/armada2-refit/installer.json`.
2. **Lists the releases** from the GitHub API at every launch and on **Refresh**. The
   list and every zip it fetches are cached in `~/.cache/armada2-refit/installer/`, so
   it works offline and keeps a release GitHub has since deleted (CI keeps only three).
   The default is the newest **full** release: a pre-release is an entry still "not yet
   seen in game", and is offered but not chosen for you. A zip is about a megabyte, so
   the chosen one is fetched straight away (checked against GitHub's sha256 digest) to
   show what it holds.
3. **Shows what it installs**, layer by layer, on this machine: the same tests
   `install.sh` makes (DXVK in the game directory for MSAA, d3d8to9 for per-pixel
   lighting, a vkBasalt layer for bloom, a `dxvk.conf` of the player's own).
   **Textures and the cutscene player are listed as not available**, switched off: a
   release cannot carry textures (they are built from the player's own files), and the
   cutscene player is held back for now (`EXCLUDED` in the script).
4. **Installs** by unpacking the zip into the cache and running its own `install.sh`
   with the game directory, `A2_PROGRESS=1` and `A2_SKIP=cutscenes`. A schema-0 zip
   ignores `A2_SKIP`, so the installer puts the stock `binkw32.dll` back afterwards, as
   `install.sh`'s `unproxy` would. It writes `armada2-refit-installed.json` into the
   game directory (version, schema, the launch variables, what it changed in Heroic).
   **Uninstall** runs the installed release's `install.sh --uninstall` (or the newest
   cached one) and takes Heroic's changes back out.
5. **Launch settings.** `install.sh` prints the variables the launcher needs
   (`WINEDLLOVERRIDES`, and bloom's two). With the switch on (default) and a Heroic entry
   for the game, they go into `GamesConfig/<app>.json` → `enviromentOptions` (Heroic's
   spelling), with the original kept once as `<file>.a2refit-backup`. A DLL override
   already there is kept and the missing ones are appended. A variable already set to
   something else is left alone and reported. A game Heroic has no settings file for yet
   is reported, not created: Heroic writes the file on first opening the game's
   settings, with defaults this program does not know. If Heroic is running it asks
   first, since Heroic may write its own copy back. Uninstall restores only values that
   are still as it left them. Any other launcher gets the text to copy.

## When there is no vkBasalt

Bloom needs a **32-bit** vkBasalt layer (the game is a 32-bit process), which no
installer here ships. When none is found, the overview shows a panel with how to get one
on this distribution, detected from `/etc/os-release`: `ID` first, then `ID_LIKE` in
order, so Mint and Pop!_OS count as Ubuntu and CachyOS, EndeavourOS and Manjaro as
Arch. `VKBASALT_HOWTO` in the script holds the instructions, and is their only copy:
`install.sh` prints them through `--vkbasalt-howto` when it skips bloom, and
`package.sh` writes all of them into `README.txt`.

| Family | Command | Why |
|---|---|---|
| Arch | `yay -S lib32-vkbasalt` | AUR only, and it pulls the AUR `vkbasalt`; needs `[multilib]` |
| Fedora | `sudo dnf install vkBasalt.i686` | in Fedora's own repositories |
| Fedora Atomic | `sudo rpm-ostree install vkBasalt.i686`, then restart | the same package, layered |
| Debian | `dpkg --add-architecture i386`, then `apt install vkbasalt:i386` | Debian builds it for i386 (bullseye on) |
| Ubuntu | build prerequisites, then `postfx/vkbasalt/build.sh` from a clone | Ubuntu builds `vkbasalt` for every architecture but i386 |
| other | the distribution's 32-bit package, or the same build | |

These were checked against each distribution's archive on 2026-10-08. **Flatpak Heroic** cannot see
a host layer, nor read the bloom folder in `~/.local/share` (it has no home access), so
the panel says bloom is not supported there yet.

## The package schema

The release zip's layout, as far as this program relies on it, is versioned on its own:
the **package schema**, an integer. `SCHEMAS` in the script lists the ones it reads; a
zip of any other schema is shown, but cannot be installed, and the window points at the
newest installer. Bump the schema only when an installer that reads the old one would
do the wrong thing with the new zip, and add it to `SCHEMAS` in the same commit. Adding
a layer or a step is not a new schema: unknown ids are shown and counted as they come.

| Schema | Zips | What the installer reads |
|---|---|---|
| 0 | 11.6.0 and older: no `manifest.json` | the layers from `README.txt` ("WHAT GOES IN", continuation lines joined), progress from `install.sh`'s ordinary output, no `A2_SKIP` |
| 1 | 11.7.0 on | `manifest.json` (below); `install.sh` honours `A2_PROGRESS` and `A2_SKIP` |

`manifest.json` (schema 1), written by `publish/package.sh`:

| Key | |
|---|---|
| `schema`, `name`, `version`, `commit` | the package |
| `installer` | `file` and `version` of the GUI installer in the same release; a newer one than the running one shows a banner |
| `linux` | `script` (`install.sh`), `uninstall` (its flag), `progress` (the variable) |
| `steps` | the `::step` ids `install.sh` prints, in order: the progress bar's denominator |
| `layers[]` | `id`, `name`, `version`, `files`, `summary`, `when`: `always`, `missing` (prerequisites), `dxvk`, `dxvk.conf`, `vkbasalt` |
| `not_included[]` | `id`, `name`, `summary`: shown as not available (textures) |

## Progress

`install.sh` with `A2_PROGRESS=1` prints `::step <id> <what>` as each stage starts
(`verify`, `prereqs`, `hud`, `menus`, `qol`, `lighting`, `online`, `msaa`, `cutscenes`,
`renderer`, `bloom`, `done`; `uninstall`). The window maps the ids to its status line
(`STATUS` in the script: "Refitting the HUD", "Installing lighting"…) and fills its
segmented bar: download 0–30 %, unpack, the script's steps 35–92 %, Heroic, done. A
schema-0 zip's stages are recognised from the lines it prints (`LEGACY_MARKS`).

## The font

The interface face is **Antonio** (SIL OFL 1.1), fetched once from Google Fonts'
repository, pinned by commit and sha256 (`FONT_URL`, `FONT_SHA`), into the cache and
loaded for this process only (`PangoCairo.FontMap.add_font_file`). Nothing is installed
system-wide. Without it, a condensed system face stands in.

## Versioning

`INSTALLER_VERSION` in the script is this layer's version. `CHANGELOG.md` heads it, and
`publish/package.sh` refuses to package if the two differ. The schemas it reads are named
in each entry. CI attaches the script to every release beside the zip, and puts a copy
in the zip.

## Testing

- `test.sh <zip>`: the self-test, then detection, install (cutscene skip, state file,
  Heroic's variables) and uninstall (game and Heroic as before) against a mock game and
  a mock Heroic in a scratch `HOME`. CI runs it on every push.
- The window was checked on a headless sway (`WLR_BACKENDS=headless`, `grim`, `wtype`).
  None of this proves anything loads in game.
