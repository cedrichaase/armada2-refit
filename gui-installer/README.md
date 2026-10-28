# gui-installer — the graphical installer, for Linux, Windows and macOS

`armada2-refit-installer.py` is one Python file: a window over a starfield that installs
a **release** of Armada II Refit into the GOG game — under Wine/Proton on Linux (GTK 4 /
libadwaita), as GOG installs it on Windows, and in a Wine prefix on macOS (both Tk). It
is for players, not for working on the project. The repository's own
`./install` builds from source and installs the asset layers too; this installs what a
release zip carries.

    python3 armada2-refit-installer.py            # the window
    python3 armada2-refit-installer.py --list     # games and releases it finds
    python3 armada2-refit-installer.py --install [VERSION] [--game DIR] [--no-launcher]
    python3 armada2-refit-installer.py --package ZIP --game DIR
    python3 armada2-refit-installer.py --uninstall [--game DIR]
    python3 armada2-refit-installer.py --setup-mac [SETUP_EXE]   # macOS, no game yet
    python3 armada2-refit-installer.py --selftest

The command line runs the same `Job` as the window and needs no GTK, which is how
`test.sh` and CI exercise it. Needs: Python 3 and its standard library; for the window,
PyGObject with GTK 4 and libadwaita (Arch `python-gobject gtk4 libadwaita`;
Debian/Ubuntu `python3-gi gir1.2-gtk-4.0 gir1.2-adw-1`; Fedora `python3-gobject gtk4
libadwaita`). Without them it says so and points at the command line.

Released as an **AppImage** for Linux (`armada2-refit-installer-x86_64.AppImage`, about
50 MB, carrying Python, GTK 4 and libadwaita), a portable **`.exe`** for Windows and a
universal2 **`.app`** for macOS (`armada2-refit-installer-macos.zip`); the script itself
is attached beside them. See "The AppImage", "Windows and macOS" and "The .exe and the
.app" below. The sections up to there describe Linux; the other two systems differ only
where "Windows and macOS" says.

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
   it works offline and keeps a release GitHub has since deleted (CI keeps only ten).
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
   (`WINEDLLOVERRIDES`). The game does not start right without
   them, so there is no switch: with a Heroic entry for the game they always go into `GamesConfig/<app>.json` → `enviromentOptions` (Heroic's
   spelling), with the original kept once as `<file>.a2refit-backup`. A DLL override
   already there is kept and the missing ones are appended. A variable already set to
   something else is left alone and reported. A game Heroic has no settings file for yet
   is reported, not created: Heroic writes the file on first opening the game's
   settings, with defaults this program does not know. If Heroic is running it asks
   first, since Heroic may write its own copy back. Uninstall restores only values that
   are still as it left them. Any other launcher gets the text to copy. The
   `ENABLE_VKBASALT` and `VKBASALT_CONFIG_FILE` an install before 13.0.0 wrote for its
   vkBasalt bloom come back out on the next install (`OLD_BLOOM_ENV`), where they are
   still as it left them: Render3D.asi blooms the 3D view itself, and both would bloom
   the frame twice.

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
| 1 | 11.7.0 on | `manifest.json` (below); `install.sh` honours `A2_PROGRESS` and `A2_SKIP`; from 12.42.0 the scripts also print `::error` lines ("Errors"), which an older installer logs as ordinary output |

`manifest.json` (schema 1), written by `publish/package.sh`:

| Key | |
|---|---|
| `schema`, `name`, `version`, `commit` | the package |
| `installer` | `file` and `version` of the GUI installer in the same release; a newer one than the running one shows a banner |
| `linux` | `script` (`install.sh`), `uninstall` (its flag), `progress` (the variable) |
| `windows`, `macos` | the same for those systems (`install.ps1`, `-Uninstall`; `install.sh`). The Windows and macOS installers install only a package that has its entry (`Package.for_system`): before 11.20.0, `install.ps1` printed no progress and `install.sh` did not run on a Mac. Adding them was not a new schema, since the Linux installer reads such a package as before |
| `steps` | the `::step` ids `install.sh` prints, in order: the progress bar's denominator |
| `layers[]` | `id`, `name`, `version`, `files`, `summary`, `when`: `always`, `missing` (prerequisites), `dxvk`, `dxvk.conf`, `d3d8to9` (Render3D, UI: needs d3d8to9 as `d3d8.dll` after the chain step, or behind Display8), from 13.35.0 `display8` (Display8: in front of d3d8to9 only; on macOS on Direct3D 9); before 13.0.0 also `vkbasalt` (bloom), which the window no longer lists |
| `not_included[]` | `id`, `name`, `summary`: shown as not available (textures) |

## Progress

`install.sh` and `install.ps1` with `A2_PROGRESS=1` print `::step <id> <what>` as each
stage starts (`verify`, `prereqs`, `chain`, `display` (from 13.35.0), `render3d`, `ui`, `qol`, `models`,
`online`, `cutscenes`, `renderer`, `bloom` (taking out the vkBasalt or ReShade bloom
of a package before 13.0.0, which set it up), `done`; `uninstall`; before 12.24.0 `hud`,
`menus` and `grid` in place of `render3d` and `ui`, and before 12.0.0 also `lighting`,
`sky` and `msaa`). The window maps the ids to its status line
(`STATUS` in the script: "Refitting the HUD", "Installing lighting"…) and fills its
segmented bar: download 0–30 %, unpack, the script's steps 35–92 %, Heroic, done. A
schema-0 zip's stages are recognised from the lines it prints (`LEGACY_MARKS`).

## Errors

A failure says what happened, the likely cause and what to do, in a dialog: the
headline, the steps, a link to the root README's "Antivirus false positives" where
that is the likely cause, the log folded away, and **Copy report**, which puts the
diagnostics ("The bug report") with this failure in them on the clipboard, ready to
paste into an issue. The command line prints the same three parts. One function makes
the words, `explain(err)` → (headline, what to do, details), and the raw message only
goes to the details and the log.

It knows a failure by a code. The release scripts (with `A2_PROGRESS=1`, from 12.42.0)
print `::error <code> <path>` on stdout before their last line, one line per file, so
the installer never parses their sentences; the installer raises `InstallError` with a
code for its own; and `error_code` maps exceptions (a bad zip, a network error, a file
refused or held open: `PermissionError`, Windows errors 5 and 32).

| Code | Meaning |
|---|---|
| `av-locked` | a file of the package cannot be read: an antivirus holds or blocks it |
| `av-missing` | a file of the package has gone: an antivirus quarantined it |
| `damaged` | a file or the zip does not match its checksum (a bad download, or an antivirus that cleans files in place) |
| `game-running` | `Armada2.exe` is running |
| `no-game` | no `Armada2.exe` in the folder given |
| `write-denied` | a file cannot be written into the game folder: an antivirus, or the folder's permissions |
| `network` | GitHub cannot be reached (the installer's own) |

The scripts check every file of `SHA256SUMS` before they stop, so every file an
antivirus took is named at once (Bitdefender took four). `Job._run_script` takes the
last `::error` line's code, with the files of every line of that code (the two `av-`
codes together). A script that prints none, a schema-0 zip or a failure with no code,
fails with its last line as before. Messages name the files that failed, never a fixed
list, so they hold when plugins are merged.

An unpack the antivirus still holds a file of cannot be removed, and it once made every
retry fail at the same point. `Package.unpack` now retries the removal with a short
backoff (about four seconds in all), then unpacks beside it, into `<version>-1`,
`<version>-2`..., and clears those spares at the next unpack, as far as it can
(`fresh_dir`, `sweep_unpacked`). A file taken or held while the zip is extracted is an
`av-` error too.

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
or Flathub. The distribution comes from `/etc/os-release` (`distro_family`): `ID` first,
then `ID_LIKE` in order, so Mint and Pop!_OS count as Ubuntu and CachyOS, EndeavourOS and
Manjaro as Arch. A game found
elsewhere (Lutris, a Wine prefix) gets the launch variables as text to copy instead.

## The bug report

**Diagnostics for a bug report** (the header menu on Linux, the **Report** link on
Windows and macOS; `--report [--game DIR]` on the command line) puts together what tells
why the mod misbehaves from a distance (`diagnose`), for the player to paste into an issue
or save and attach. The installer is the place for it because it runs where the game
draws nothing, and it already knows the game folder, the launcher and the release.

- **Files by hash, never the files.** `d3d8.dll`, `d3d9.dll`, `winmm.dll` and
  `binkw32.dll` are named by sha256 against what a release puts in (`KNOWN_FILES`, from
  `platform/vendor/*/SOURCE.txt`, and GOG's own `d3d8.dll`), else by what they say of
  themselves (Display8, ours, by its panic message, with `d3d8_real.dll` behind it; DXVK,
  Wine's builtin, dgVoodoo). Asking a player to post the DLL itself
  could mean posting GOG's or Microsoft's, which may not be redistributed; the hash says
  as much. Every `.asi` is checked against the release named in the state file, read from
  the installer's cache, and the `.ini` keys that differ from that release are listed.
- **The logs are cut.** Each plugin rewrites its log at every launch, but `UI.log` and
  `Render3D.log` still run to tens of thousands of lines, so the report keeps the head,
  the lines in between that say something stood down or failed or which way the menu
  scene went (`LOG_KEY`, each shape once, digits aside) and the tail. QOL's logs append
  across launches; their tail is the last one.
- **Findings come first**: the causes known so far, each tested on the files and the whole
  logs. Among them: UI.asi or Render3D.asi without crosire's d3d8to9 (no Direct3D 9
  device, so both stand down; GOG's `d3d8.dll` back after a Galaxy repair, or DXVK's put
  in by Heroic), the launcher not loading the folder's `winmm`/`d3d8`, UI.asi beside the
  plugins it supersedes, and `Menus.asi` drawing without a widescreen plate, which
  shows black at the sides of the main menu (the release carries no plates). First of
  all: files of a release missing or unreadable in the installer's cache (each file of
  the newest unpack of each version, against its zip) or in the game folder, as an
  antivirus leaves them.
- **Safe to post** (`redact`): the home folder becomes `~`; any other user folder
  (`C:\Users\name`, `/home/name`, `/Users/name`, Wine's `Z:\home\name`) loses the name;
  IPv4 addresses (Online.log's peers) become `<ip>`.

Copy puts it in a code block, so GitHub and Discord keep its lines.


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
- fontconfig and freetype are the host's, as the AppImage project's own exclude list
  has them. A bundled fontconfig reads the host's config files, and one older than the
  host's (Ubuntu 24.04's 2.15 under Arch's newer one) printed about ninety "invalid
  constant" and `xsi:nil` warnings at every start. Every graphical Linux has both.
- A bundled Python's compiled-in certificate folder may not exist on the host, so
  `ssl_context()` falls back to the usual bundle files.
- `appimagetool` and the type-2 runtime are fetched by `build.sh`, pinned by version and
  sha256, into `~/.cache/armada2-refit/appimage-tools`. Nothing from there is committed;
  the AppImage itself is a release asset, never a file in the repository.
- `A2_INSTALLER_APPIMAGE=<file> test.sh <zip>` runs the whole mock-game test through the
  AppImage. Where FUSE is missing, `APPIMAGE_EXTRACT_AND_RUN=1` runs it without
  mounting (the type-2 runtime is static, so no libfuse2 is needed to mount either).

## Windows and macOS

The core is the same on all three systems (`SYSTEM`); what differs:

| | Windows | macOS |
|---|---|---|
| The game | GOG's own install: the folder in GOG's registry entry (`HKLM\SOFTWARE\[WOW6432Node\]GOG.com\Games\1174788223`, `gog_registry_dirs`), else the usual folders on every drive | inside a Wine prefix (`mac_prefixes`): CrossOver bottles (`~/Library/Application Support/CrossOver/Bottles`), Whisky's, Sikarugir / Kegworks / Wineskin / Porting Kit wrappers (the prefix inside the `.app`), `~/.wine` and the like |
| The script | `install.ps1` through `powershell.exe -ExecutionPolicy Bypass -File`, no console window | `install.sh` through `/bin/bash` (3.2, BSD tools) |
| The chain | crosire's d3d8to9 on Windows' own Direct3D 9, with Display8 in front (stub mode, `Backend=auto`); a DXVK `d3d8.dll` the player put there stays, without Display8 | d3d8to9 on the prefix's own Direct3D 9 (wined3d, or a Sikarugir wrapper's D9VK): DXVK's `d3d9.dll` cannot run on a Mac, so no MSAA or `dxvk.conf`; Display8 in front on `Backend=auto`, Vulkan on MoltenVK first |
| Launch settings | none: the game folder's DLLs load first | the DLL overrides, written into the prefix's `user.reg` under `AppDefaults\Armada2.exe\DllOverrides` (the game's alone, not the bottle's) |
| Launch game | the exe | the prefix's Wine: CrossOver's `wine --bottle <name>`, a wrapper's own (with the wrapper's `Frameworks` on `DYLD_FALLBACK_LIBRARY_PATH`, `wrapper_env`), Whisky's, else Gcenx's or Homebrew's |
| Cache, settings | `%LOCALAPPDATA%\armada2-refit\installer\cache`, `%APPDATA%\armada2-refit\installer.json` | `~/Library/Caches/armada2-refit/installer`, `~/Library/Application Support/armada2-refit/installer.json` |

**The registry, on macOS** (`prefix_plan`, `prefix_apply`). Only the overrides the game
does not have yet go in, as Heroic's do on Linux; the first write keeps
`user.reg.a2refit-backup`, and uninstall takes out only values still as it left them,
with the section if it held nothing else, so the file comes back byte for byte. Wine
must not be running in the prefix: wineserver writes its own copy back on exit, so with
a `wineserver` running nothing is written and the player is told. **Launch game** passes
the same overrides in the environment as well, so it works either way. A game outside
any prefix gets the variables as text to copy.

**Python.** The SOD rewrites are Python; Windows and macOS players have none (and
macOS's `/usr/bin/python3` is a stub that offers to install the developer tools). The
frozen program runs a `.py` given as its first argument (`run_python`), and the job sets
`A2_PYTHON` to itself; both scripts use `A2_PYTHON` when it is set. An exception there
goes to stderr, never to a dialog that would stall the script.

**Which stack on a Mac.** The window sets the game up in a **Sikarugir** wrapper itself
(below), free and seen in game. **CrossOver 26** (CodeWeavers, paid; Wine 11, rated
"Runs Great" for Armada II) works too: the game installed into a bottle is found like
any other. **Gcenx's** macOS Wine builds are the third way. Whisky was discontinued in 2025, and Homebrew's `wine-stable` cask is
disabled. Every Wine on Apple Silicon runs under Rosetta 2, which Apple keeps fully
through macOS 27 and then only "for older unmaintained gaming titles"; whether Wine
stays inside that is not announced. The window's own text for this is `MAC_HOWTO` in the
script, its only copy (and `WINDOWS_HOWTO` for getting the game on Windows).

### Setting the game up on a Mac

With no game found on macOS, the window's button is **Set up Armada II** (`--setup-mac`
on the command line), enabled once GOG's Windows offline installer is in `~/Downloads`
(`find_setup_exe`: `setup_star_trek*armada*.exe`, the newest; the window says whether it
found one). GOG's installer cannot be fetched for the player: it needs their GOG
account, so `MAC_HOWTO` says where in the GOG library it is. After a yes to a dialog
that lists what follows, `Job.setup_mac` does what Sikarugir Creator and the wrapper's
*Install Software* would make the player click through:

1. **Rosetta 2**, if `/Library/Apple/usr/share/rosetta/rosetta` is missing:
   `softwareupdate --install-rosetta --agree-to-license` (the dialog says it agrees to
   Apple's licence). Every Wine on Apple Silicon runs under it.
2. **Sikarugir's template and engine** from Sikarugir's own GitHub releases, pinned by
   URL, size and sha256 (`SIKARUGIR_TEMPLATE`: `Template-1.0.24`, `SIKARUGIR_ENGINE`:
   `WS12WineSikarugir11.0_1`, Wine 11 in new-style WoW64), cached in
   `~/Library/Caches/armada2-refit/installer/sikarugir/`. They are fetched, never
   shipped: the template and its launcher are not under the LGPL, and the refit
   carries neither.
3. **The wrapper**, built in `~/Applications/Sikarugir/.armada-ii-setup` and renamed to
   `Armada II.app` only once the game is in it, so a setup that stops leaves no wrapper
   for the search to find. It is what Creator makes, checked file by file against a
   Creator-made one: the template unpacked, the engine's `wswine.bundle` as
   `Contents/SharedSupport/wine`, and three Info.plist keys (`wrapper_plist`:
   `CFBundleIdentifier`, `CFBundleName`, `Program Name and Path`, the exe in
   `drive_c`); the rest of the template's settings stay, D9VK on among them, which the
   game does not use with Display8 on Vulkan.
4. **The prefix**, by the wrapper's own launcher, as Creator makes it:
   `Contents/MacOS/wineskinlauncher WSS-wineprefixcreate` (Sikarugir's environment and
   its default registry, which a bare `wineboot` lacks). The launcher is one binary
   under three names (`Sikarugir`, `launcher`, `wineskinlauncher`); the `sikarugir
   create-prefix` / `run` usage in its strings is not answered inside a wrapper. It may
   return before Wine is done, so `_wait_wine` waits until no process of the wrapper's
   Wine is left. The engine carries Wine Mono and Gecko, so nothing asks to download
   them.
5. **The game**: GOG's installer is Inno Setup's, run `/VERYSILENT /SUPPRESSMSGBOXES
   /NORESTART /SP-` into GOG's own default folder, `C:\GOG Games\Star Trek Armada II`.
   The launcher passes flags only to the wrapper's own program, so for this one run
   `Program Name and Path` is a copy of the installer at `C:\a2refit-setup\setup.exe`
   (an APFS clone, `cp -c`) and `Program Flags` Inno's; the launcher is started without
   arguments, and the copy goes and `Armada2.exe` becomes the program again afterwards.
   No `Armada2.exe` there is an error that names the installer.

The window then finds the game in the new wrapper and installs the chosen release into
it as usual (the DLL overrides into its `user.reg`). The player starts it with
`~/Applications/Sikarugir/Armada II.app` or **Launch game**. The app itself stays
unnotarised: its first launch still needs *Open Anyway*.

### The window on Windows and macOS

The `.exe` and the `.app` show the same one screen in Tk (8.6 or 9, from the standard
library), in `run_tk()`. Everything is drawn on a single Canvas — the sky, the
translucent card, the buttons and the pickers — since a Tk widget is opaque and would
cut a box out of the sky. The card, buttons and fields are antialiased rounded
rectangles with an alpha channel, built as PNGs in Python (`rounded_rows`, `tk_png`) and
blended by Tk over the canvas; the pickers open a menu on Windows, and on macOS a list
drawn on the canvas too (`open_picker`): Tk's native popup menu (8.6.16), posted from a
click on the canvas, left the whole window deaf to the mouse there until it was killed.

The sky is `Sky` redone for a Canvas: the same `nebula_pixels`, composited over the
gradient and scaled to the window height in a thread (`nebula_rgb`, a third of a second),
faded in over five steps and then moved only by whole pixels; each star layer moves as
one canvas tag, and only when it has travelled a whole pixel. At rest that is about 5 %
of a core. While a job runs it goes to warp as on Linux, with tails stippled so the sky
shows through (macOS draws them solid).

Jobs run on a worker thread and reach Tk only through a queue polled every 40 ms. Errors
in the window's own code go into the log (Ctrl/Cmd+L), since the frozen program has no
console. On Windows it sets per-monitor DPI awareness and sizes everything by
`tk scaling`; on macOS Tk's pixels are already points. Keys: F5 refresh, Ctrl+Enter
(Cmd+Return) install, Ctrl+L (Cmd+L) the log. `A2_INSTALLER_TK=1` opens this window on
Linux, to work on it; it was checked there on a headless sway with Xwayland, with Tk 8.6
and 9, clicks synthesised with `event_generate`.

## The .exe and the .app

`pyinstaller/build.py [--out DIR] [--arch universal2]` freezes the script with
PyInstaller for the system it runs on, pinned in `pyinstaller/requirements.txt` (build.py
refuses another version). CI builds both:

- **Windows**: `armada2-refit-installer-windows.exe`, one windowed file, portable
  (nothing to install, no Python needed), with the icon and a version resource carrying
  `INSTALLER_VERSION`. Unsigned: SmartScreen may hold it the first time (*More info* →
  *Run anyway*). 64-bit; the game's 32 bits do not matter to it.
- **macOS**: `Armada2 Refit Installer.app`, universal2 (Apple Silicon and Intel), built
  on macos-15 with setup-python's universal2 framework Python (with Tk), signed ad hoc
  and not notarised, zipped with `ditto` into `armada2-refit-installer-macos.zip`. The
  first launch is refused: on macOS 15 and later, System Settings → Privacy & Security
  → *Open Anyway*; on older ones, right-click → *Open*. Or
  `xattr -dr com.apple.quarantine "Armada2 Refit Installer.app"`.

Both run in Python's UTF-8 mode (`--python-option "X utf8"`). The modules the SOD scripts
import are named as hidden imports in build.py. The icon is drawn from
`appimage/armada2-refit.svg` by `pyinstaller/icon.py` in pure Python; it draws only the
SVG features the icon uses and refuses anything else, so a richer icon fails the build
rather than drawing wrong.

## Versioning

`INSTALLER_VERSION` in the script is this layer's version. `CHANGELOG.md` heads it, and
`publish/package.sh` refuses to package if the two differ. The schemas it reads are named
in each entry. CI attaches the script to every release beside the zip, and puts a copy
in the zip.

## Testing

- `test.sh <zip>`: the self-test (with `explain` for every code), then detection, install
  (cutscene skip, state file, Heroic's variables), the bug report on that install, a
  package missing two plugins (the words, both files named, the report's antivirus
  finding), `_run_script` on `::error` lines and on a schema-0 script's last line, an
  unpack that cannot be cleared, and uninstall (game and Heroic as before) against a
  mock game and a mock Heroic in a scratch `HOME`. CI runs it on every push.
- `A2_INSTALLER_APPIMAGE=<file> test.sh <zip>`: the same through the AppImage. CI does it.
- `test-windows.ps1 <zip> <exe>`: the shipping `.exe`, its standard handles redirected
  (it is windowed), in scratch `LOCALAPPDATA`/`APPDATA`/`USERPROFILE`: version resource,
  self-test, the exe as Python, install (cutscene skip, state file), a second install
  while a file of the first unpack is held open (it goes beside it), and uninstall (the
  game as before). CI runs it on windows-2025.
- `test-macos.sh <zip> <app or its zip>`: the `.app` in a scratch `HOME` with a mock
  CrossOver bottle: detection, the app as Python, install, the DLL overrides written into
  the bottle's `user.reg` and taken out byte for byte, uninstall. CI runs it on
  macos-15, after `publish/installer/test.sh`.
- The window was checked on a headless sway (`WLR_BACKENDS=headless`, `grim`, `wtype`).
  None of this proves anything loads in game, and nothing here has been run on a real
  Windows or Mac desktop by a person yet: only on CI's runners.
