# gui-installer — the graphical installer for Linux

`armada2-refit-installer.py` is one Python file: a GTK 4 / libadwaita window over a
starfield that installs a **release** of Armada II Refit into the GOG game under
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

Released as an **AppImage** (`armada2-refit-installer-x86_64.AppImage`, about 50 MB), which
carries Python, GTK 4 and libadwaita; the script itself is attached beside it. See
"The AppImage" below.

Keys: F5 refresh, Ctrl+Enter install, Ctrl+L the log.

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
3. **Says what will not go in**, on this machine, in a line each under the card: the same
   tests `install.sh` makes (DXVK in the game directory for MSAA, d3d8to9 for per-pixel
   lighting, a `dxvk.conf` of the player's own; `layer_verdict` in the script). There is
   no layer table: what installs is the release's business. **Textures and the cutscene
   player are not installed from here** (`EXCLUDED`): a release cannot carry textures,
   and the cutscene player is held back for now.
4. **Installs** by unpacking the zip into the cache and running its own `install.sh`
   with the game directory, `A2_PROGRESS=1` and `A2_SKIP=cutscenes`. A schema-0 zip
   ignores `A2_SKIP`, so the installer puts the stock `binkw32.dll` back afterwards, as
   `install.sh`'s `unproxy` would. It writes `armada2-refit-installed.json` into the
   game directory (version, schema, the launch variables, what it changed in Heroic).
   **Uninstall** runs the installed release's `install.sh --uninstall` (or the newest
   cached one) and takes Heroic's changes back out.
5. **Launch settings.** `install.sh` prints the variables the launcher needs
   (`WINEDLLOVERRIDES`, and bloom's two). The game does not start right without
   them, so there is no switch: with a Heroic entry for the game they always go into `GamesConfig/<app>.json` → `enviromentOptions` (Heroic's
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

## The window

One screen: the game, the version, a line saying what goes into Heroic, **Install** (and
**Uninstall** when the game has a release in it), a thin progress bar and a status line.
The log is a dialog (`Ctrl+L`). The sky behind it is drawn in code (`Sky`): a gradient,
a nebula and three layers of stars drifting at different speeds, at about 30 frames a
second, still when the desktop has animations off. The nebula is new at every launch
(`nebula_pixels`): value noise, gently domain-warped into soft billows (thin filaments
looked electric), with dust lanes and a hue moving between violet and teal. It is pure
Python, since the AppImage carries no numpy, so a thread computes it at 256x160 (about
half a second) and it fades in. It is scaled to the window once per height and then only
moved by whole pixels, at 1.5 px/s behind the slowest stars: filtering it every frame
cost half as much CPU again as the rest of the sky. Brightness is set by a percentile,
not the brightest cell, so every launch comes out about as bright and nothing clips
behind the cards. Nothing is shipped or fetched for it, so the script stays one file and
`publish/check.sh` has nothing to refuse. The face is the desktop's own. The window uses
libadwaita 1.5 at most (`AlertDialog`, `Dialog`).

While an install runs the sky goes to warp (`Sky.set_warp`): everything travels up to 40
times faster and the stars draw as streaks, faint at the tail, each at least as long as
it moves in a frame so it reads as one motion. It eases in and out over about a second
and holds for at least three, since an install from the cache takes under one. Positions
follow a distance integrated over time, not the clock, so a change of speed never makes
anything jump; at rest it costs nothing, and at warp the sky draws every frame instead
of thirty a second.

The accent is amber (`AMBER`, the warm of the nebula's cores), not the desktop's:
libadwaita's accent colours by their old names and, on GTK 4.16 and later, the CSS
variables libadwaita 1.6 reads them from, plus the Install button and the progress bar
set outright, so those two hold on any libadwaita.

The title is "ARMADA II" at weight 900 over a small, widely tracked amber "REFIT"
(`.title-main`, `.title-tag`), in the desktop's own face: no font is shipped or fetched,
and every common desktop sans (Adwaita Sans, Cantarell, Noto Sans, Ubuntu) has the heavy
weight. The tag's margin makes up for the tracking GTK also puts after its last letter,
which would otherwise push it off centre.

**Launch game** sits beside Install once the game has a release in it and Heroic knows
it. It opens `heroic://launch?appName=<app>&runner=gog`, the URI Heroic 2.x's own
desktop shortcuts use, through `xdg-open` with `host_env()`, so Heroic never inherits
the AppImage's libraries (GTK's URI launcher is only the fallback). The amber goes to
whichever of the two there is to do: Install while something new is there, Launch once
the button would only say Reinstall.

The progress bar (`EngineBar`) has an engine at its tip: a flickering amber glow and a
short trail, drawn on an overlay 24 px high around the 4 px bar. The bar glides to each
new fraction instead of jumping step to step, and the glow fades out once it is full.

When there is no Heroic at all (no config folder, no `heroic` on the `PATH`, no Flatpak)
and the game is not one Heroic knows, an open row says how to get it for this
distribution (`HEROIC_HOWTO`): the AUR package on Arch, otherwise the site's `.deb`/`.rpm`
or Flathub, with the warning that bloom does not work in the Flatpak. A game found
elsewhere (Lutris, a Wine prefix) gets the launch variables as text to copy instead.


## The AppImage

`appimage/build.sh [OUT]` builds it; CI runs that on **ubuntu-24.04**, which sets the
oldest C library it runs on (glibc 2.39: Ubuntu 24.04, Debian 13, Fedora 40, and newer).
It is not a container image and needs no root, but it assembles from the system it runs
on, so build it where you mean it to be portable.

- `appimage/build.py` copies the running Python and its standard library, PyGObject and
  pycairo, GTK 4, libadwaita and every library they load (`ldd` over each, less what the
  host must provide), the typelibs, compiled GLib schemas, Adwaita icons and gdk-pixbuf's
  loaders into an AppDir. **Left to the host on purpose:** the C library and its
  companions, libstdc++, everything that talks to the graphics driver (GL, EGL,
  DRM, GBM) and the display libraries (Wayland, X11, xcb): bundling those breaks the host's
  Mesa. Small libraries a host may lack (`libselinux`, `libsystemd`, `libxkbcommon`, the
  Vulkan loader) are bundled: leaving `libselinux` out once broke Ubuntu's GLib on a host
  without it. `build.py` prints "left to the host"; read that list after touching the
  exclusions. There is no dconf module, so GSettings uses its memory backend.
- `appimage/AppRun` points Python, GTK and the loader at the bundle, and **stores every
  variable it changes in `A2_ORIG_<NAME>`** (`:unset` for none). `host_env()` in the script
  puts them back for every program the installer starts: `install.sh` and the `bash` and
  `python3` it calls must see the host's `LD_LIBRARY_PATH`, `PYTHONHOME` and `PATH`, not
  the bundle's. This is the part most likely to break silently.
- On a host with a newer fontconfig than the bundle's, the bundled one prints "invalid
  constant" warnings about the host's config files at start. They are harmless.
- A bundled Python's compiled-in certificate folder may not exist on the host, so
  `ssl_context()` falls back to the usual bundle files.
- `appimagetool` and the type-2 runtime are fetched by `build.sh`, pinned by version and
  sha256, into `~/.cache/armada2-refit/appimage-tools`. Nothing from there is committed;
  the AppImage itself is a release asset, never a file in the repository.
- `A2_INSTALLER_APPIMAGE=<file> test.sh <zip>` runs the whole mock-game test through the
  AppImage. Where FUSE is missing, `APPIMAGE_EXTRACT_AND_RUN=1` runs it without
  mounting (the type-2 runtime is static, so no libfuse2 is needed to mount either).

## Versioning

`INSTALLER_VERSION` in the script is this layer's version. `CHANGELOG.md` heads it, and
`publish/package.sh` refuses to package if the two differ. The schemas it reads are named
in each entry. CI attaches the script to every release beside the zip, and puts a copy
in the zip.

## Testing

- `test.sh <zip>`: the self-test, then detection, install (cutscene skip, state file,
  Heroic's variables) and uninstall (game and Heroic as before) against a mock game and
  a mock Heroic in a scratch `HOME`. CI runs it on every push.
- `A2_INSTALLER_APPIMAGE=<file> test.sh <zip>`: the same through the AppImage. CI does it.
- The window was checked on a headless sway (`WLR_BACKENDS=headless`, `grim`, `wtype`).
  None of this proves anything loads in game.
