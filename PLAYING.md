# Playing with Armada II Refit

For players. It covers installing, what each part does, how to switch parts off, the
Refit screen, multiplayer, what to do when something goes wrong and how to take it all
out again. For how the parts work inside, see the READMEs the links point to; for
building your own mod, [`MODDING.md`](MODDING.md).

Refit changes how the game looks and handles. It does not change what a mission is:
the campaigns, maps, ships and balance are Armada II's. One part, *Pay when queued*,
changes a rule of network games, and says so (see "Multiplayer").

## What you need

- **Star Trek: Armada II, the GOG release, patch 1.1.** Other copies are not
  recognised: the plugins check the game's executable before they change anything,
  and on one they do not know they patch nothing and say so in their log.
- A graphics driver for **Vulkan or Direct3D 12**. Almost any card from the last ten
  years has one. Without either, the game still starts, on the system's own Direct3D 8,
  and looks as stock (see "Choosing a graphics backend").
- Linux with Heroic and Proton, Windows 10 or 11, or macOS with a Wine of some kind.
  Linux is where the project is developed and tested; Windows and macOS are supported
  by the installer but have had less play (the release notes say what has been seen in
  game).

Textures, menu backdrops and replacement movies are not in a release. They are built
from your own copy of the game with the repository's tools, because a release cannot
carry the game's art ([`textures/PACKS.md`](textures/PACKS.md)). Everything below works
without them.

## Installing

Use the graphical installer from the newest
[release](https://github.com/cedrichaase/armada2-refit/releases). It finds the game,
shows what it will install, installs, and on Linux and macOS sets the launch options
the game needs. A release zip also contains scripts (`install.sh`, `install.bat`) that
do the same from a terminal; the installer runs them for you
([`gui-installer/README.md`](gui-installer/README.md)).

| System | Download | Notes |
|---|---|---|
| Linux | `armada2-refit-installer-x86_64.AppImage` | Make it executable and run it. It carries GTK 4 and libadwaita. It finds the game in Heroic's list, Lutris and the usual Wine prefixes, and writes the DLL overrides into Heroic's settings for the game (a copy of the original is kept as `<file>.a2refit-backup`). If Heroic is running, it asks first. For another launcher it prints the text to copy. |
| Windows | `armada2-refit-installer-windows.exe` | One portable file. GOG's install is found through its registry entry. It is not code-signed, so SmartScreen may ask: *More info*, then *Run anyway*. No launch options are needed. |
| macOS | `armada2-refit-installer-macos.zip` | An app for Apple Silicon and Intel. The game runs in Wine. With no game yet, the app sets it up for you from GOG's Windows offline installer in a free Sikarugir wrapper, then installs the refit; a game already in CrossOver or another wrapper is found as it is. The app is not notarised, so the first launch needs *Open Anyway*. Every step: [On a Mac, step by step](#on-a-mac-step-by-step). |

Three things to know before you click:

- **The installer needs the game closed**, and Heroic closed when it writes Heroic's
  settings.
- **Pre-release (latest main).** The installer's *Version* list ends with a
  *Pre-release (latest main)* entry: the newest build of the repository's `main`,
  rebuilt on every change and not seen in game. It is never chosen for you; pick it to
  help test, or to live on the edge. A version marked *untested release* is a numbered
  release that has not been seen in game yet; it is not chosen for you either. To go
  back, pick a release from the same list and install again (or `--install VERSION`
  from a terminal; `--install pre-release` takes the latest-main build).
- **What it adds.** In the game folder: `Display8` (as `d3d8.dll`), the plugins
  (`UI.asi`, `Render3D.asi`, `QOL.asi`, `QOLRules.asi`, `Online.asi`), their `.ini`
  files, the ASI loader (`winmm.dll`) and the widescreen patch if they are missing,
  and a replacement `binkw32.dll` that keeps the stock one beside it. Your own
  `.ini` edits are merged, never replaced: keys that are new are added, your values stay.
  A `d3d8.dll` of GOG's is kept once as `d3d8.dll.gog-backup`.
- **A `d3d8.dll` of your own** (a wrapper you chose on purpose) is left alone. Then
  `Render3D.asi` and `UI.asi` are not installed, since they draw on Display8's device,
  and the game keeps its own look. The installer says so in a line under the release.
- **Earlier versions' leftovers** (the old HUD, menu and grid plugins, the Direct3D 9
  chain, `dxvk.conf`) are taken out by the same install; a file of yours that is none
  of those stays.

If your antivirus quarantines a plugin or holds it while the installer copies, that is
a false positive of unsigned DLLs that hook a program. The README's
[Antivirus false positives](README.md#antivirus-false-positives) says what to restore
and exclude.

### Leaving a part out

The installer's *Parts* list has a tick for every layer that is a choice. Untick one
and it is not installed, or is taken out if an earlier install put it there; the choice
is remembered for upgrades. From a terminal, `install.sh --without qol,rules` does the
same.

| Part | File(s) | What it does | Leave it out and... |
|---|---|---|---|
| Display8 | `d3d8.dll` | The game's Direct3D 8, ours. Draws on Vulkan or Direct3D 12, and hands the game the system's own Direct3D 8 when neither starts | (not a choice: the next two need it) |
| 3D view | `Render3D.asi` | The 3D view through a renderer of our own: anti-aliasing, bloom, lit ships with shadows, skies, planets, nebulae you can fly through, effects, fog of war with soft edges | the game draws its own 3D view |
| HUD and menus | `UI.asi` | The HUD, the font, the cursor, the menus, the loading screen and the Refit screen, at any aspect ratio | stock HUD and menus, stretched or boxed at a wide screen, and **no Refit screen** |
| Quality of life | `QOL.asi` | Faster right-drag panning, `Shift+number` adds to a group, selections and groups beyond 16, stations in control groups, long moves at warp. Works with players who do not have it | stock handling |
| Rules | `QOLRules.asi` | *Pay when queued*. Every player of a network game needs it (see "Multiplayer") | stock payment |
| Online | `Online.asi` | The *Internet - Online* entry in Multiplayer | the entry is not there |
| Cutscene player | `binkw32.dll` | The launch reels full screen (and AV1 replacements for the movies, if you have built any and put them beside the game) | stock player |
| Models | SOD files | Smoothed moons and asteroids, a round selection bubble, a widened loading screen. Rewritten from your own files | stock models |

You can also switch most of these off **without reinstalling**, from the Refit screen
(next section): that gives the stock look for that one part, on any backend.

### On a Mac, step by step

Armada II is a Windows game, so on a Mac it runs in Wine. The installer app sets all of
it up: you bring GOG's installer for the game, it does the rest.

**Known working:** an Apple M1 on macOS 27.0.1, the game set up by the app in a
Sikarugir wrapper (Wine 11, engine `WS12WineSikarugir11.0_1`), Display8 drawing on
Vulkan through MoltenVK (`Display8.log`: `vulkan: physical device Apple M1`). Intel
Macs and CrossOver are supported by the app but have not been seen in game by the
project yet; if you play on one, a line in an issue saying how it went helps.

**What you need:** the game in your GOG account, a connection to the internet, and a
few GB free (the Wine engine and its wrapper alone are about 260 MB to download).

1. **Get the installer app.** From the newest
   [release](https://github.com/cedrichaase/armada2-refit/releases), download
   `armada2-refit-installer-macos.zip` and open it; Finder unpacks
   `Armada2 Refit Installer.app`. Move it to *Applications* if you like.
2. **Open it the first time.** The app is not notarised by Apple, so the first double
   click is refused. On macOS 15 and later: close that message, open *System Settings*,
   *Privacy & Security*, scroll down to the line about the app and press *Open Anyway*,
   then confirm. On older macOS: Control-click the app, *Open*, *Open*. (Or, in
   Terminal: `xattr -dr com.apple.quarantine "Armada2 Refit Installer.app"`.) Later
   launches open normally.
3. **Download the game's Windows installer from GOG.** Sign in at
   [gog.com](https://www.gog.com/account), open Star Trek: Armada II in your library,
   and under *Download offline backup game installers* take the **Windows** one
   (`setup_star_trek_armada_ii_….exe`). Leave it in your *Downloads* folder: that is
   where the app looks. Not GOG Galaxy, and not a patch or extra.
4. **Set up the game.** In the app, the *Game* row says it found none, and below it
   *Found in Downloads:* with the installer's name (press *Refresh* if you downloaded it
   after opening the app). Untick any part you do not want under *Parts*, then press
   **Set up Armada II** and say yes to the list of what it will do. It then, on its own:
   installs Rosetta 2 on Apple Silicon if it is missing (Apple's translator, which every
   Wine on those Macs needs); downloads Sikarugir's wrapper and Wine engine from
   GitHub; makes `~/Applications/Sikarugir/Armada II.app`; runs GOG's installer inside
   it without any windows; and installs the refit into it, with the Wine settings the
   plugins need. This takes a few minutes; the bar and the log (Cmd+L) show where it is.
5. **Play.** Press **Launch game**, or open *Armada II* in `Applications/Sikarugir` in
   your home folder (Finder: *Go*, *Home*). You can drag that one to the Dock. The
   game's *Options* menu has a **Refit** entry when the plugins loaded.
6. **Upgrade** later by opening the installer app again: it finds the game in its
   wrapper and offers *Upgrade to* the newest release. Your settings and saved games
   stay.

**Already have the game in CrossOver, or in another wrapper** (Kegworks, Wineskin,
Porting Kit, Whisky)? Open the app: it finds the game there and the button is
*Install*. To put it into CrossOver, install it into a bottle with the same GOG
installer (*Install a Windows Application*, then choose the `.exe`), then press
*Refresh* in the app. Leave the bottle's graphics backend on its default for this game:
D3DMetal, DXVK and DXMT only cover Direct3D 10 to 12, and Armada II is Direct3D 8.
Quit the game and the bottle's programs before you install, or the Wine settings
cannot be written (the app says so).

**When the setup stops** it leaves no half-made wrapper behind, and says why:

| The app says | What to do |
|---|---|
| *Set up Armada II* is greyed out | No installer in Downloads (step 3), or no release could be fetched: check the connection and press *Refresh* |
| `Armada II.app is there already, without the game in it` | Move `~/Applications/Sikarugir/Armada II.app` to the Bin and set up again |
| `Rosetta 2 could not be installed` | In Terminal: `softwareupdate --install-rosetta --agree-to-license`, then set up again |
| `GOG's installer did not install the game` | The file in Downloads is not the Windows offline installer of the game (a Mac file, a patch, GOG Galaxy). Download the right one (step 3) |
| The game starts but has no Refit entry | Wine was running while the app wrote its settings. Quit the game, press *Reinstall*; *Launch game* passes the settings anyway |

The logs (see "The logs" below) are inside the wrapper: Control-click
`Armada II.app`, *Show Package Contents*, then `Contents/SharedSupport/prefix/drive_c/users/`,
your user, `AppData/Local/armada2-refit/logs/`. The installer's **Report** collects them
for you.

**Removing it.** *Uninstall* in the app takes the refit out and leaves the game. To
remove the game too, move `~/Applications/Sikarugir/Armada II.app` to the Bin; your
saved games are in the game's folder inside it (*Show Package Contents*, as for the
logs, then `Contents/SharedSupport/prefix/drive_c/GOG Games/Star Trek Armada II`), so
copy them out first if you want to keep them. The app's downloads are in `~/Library/Caches/armada2-refit/` and its settings in
`~/Library/Application Support/armada2-refit/`.

## The Refit screen

In the main menu or in a mission: **Options**, then **Refit settings**. It lists every setting
of every Refit plugin that is installed, on three tabs: Graphics, Interface and Gameplay. A click anywhere on an on/off row toggles it; a row with
several values opens a list. *Show advanced* adds the rows meant for tuning or trouble.
*Reset tab* removes your choices on that tab and goes back to the shipped defaults
(it asks first). *Done* or Esc closes the screen.

Each row has a badge saying when a change takes hold:

| Badge | Meaning |
|---|---|
| Now | applied at once, in the menu or in the mission you are playing |
| Next mission | applied when the next mission starts (or a save is loaded) |
| Restart | applied when the game is started again. After you change it the badge turns to *Restart needed* |
| Must match | every player of a network game must have the same value |
| Locked | cannot be changed now: a must-match row during a network game |

A row drawn dimmed with a line of text means that part is not running right now and says
why ("Stands down: ... is not stock in this Armada2.exe", or a plugin that is not
loaded). Your choice is kept and applies when the part runs.

The screen only ever writes one file, `RefitPlayer.ini`.

### `RefitPlayer.ini`

Your choices live in `RefitPlayer.ini`, in the game folder beside `Armada2.exe`. It is
yours: **no installer, upgrade, uninstall or release ever writes, replaces or deletes
it**, so a reinstall keeps your settings. Only choices that differ from the default are
stored, and setting a row back to its default removes its line.

Each section is `[<plugin ini's name>.<section>]` and the keys are spelled as in that
plugin's own `.ini`:

    [Render3D.Modules]
    hulls=0
    [UI.HUD]
    Minimap=0
    [QOLRules.Rules]
    PayOnQueue=0

A key is read from `RefitPlayer.ini` first, then from the plugin's own `.ini` (the
tuning values, which the installer ships and merges), then the built-in default. You
can edit the file by hand while the game runs: Refit looks at it about once a second
and applies live rows. Without `UI.asi` there is no Refit screen and no live
re-reading; edits apply at the next launch.

**On Windows, with the game under `Program Files`**, Windows redirects writes of a
program there to `%LOCALAPPDATA%\VirtualStore\Program Files (x86)\...\Star Trek Armada
II\` (the game folder's path under the same root). The game does not notice, but when
you look for the file in Explorer it may be in that folder rather than the game's.
The installer's report looks in both. Moving the game outside `Program Files`
avoids the question.

### Tuning beyond the screen

The screen holds the choices a player makes: parts on or off, anti-aliasing, view
distance, a few gameplay keys. Fine tuning (bloom strength, exposure, the look of
the shields, sky recipes) stays in the plugin `.ini` files in the game folder, each
key commented ([`render3d/Render3D.ini`](render3d/Render3D.ini),
[`ui/UI.ini`](ui/UI.ini), [`qol/QOL.ini`](qol/QOL.ini)). The same overlay rule
applies, so a value you set in `RefitPlayer.ini` wins over one in the plugin's file.

## Multiplayer

Multiplayer under Wine and Proton does not work with the stock game at all, LAN
included (Wine stubs DirectPlay). Refit's *Internet - Online* entry replaces the
transport.

- **Internet - Online** takes the place of the IPX button on the Multiplayer
  Connection screen. The host gets a short join code; the others type it. Players
  connect directly when their routers allow, and through the project's relay when they
  do not, so no port forwarding is needed. The server is `c20e.de` by default and can be
  self-hosted ([`online/README.md`](online/README.md)). Every other connection type
  keeps the game's own DirectPlay.
- **Playing against players without Refit** works for everything that stays on your
  machine: the HUD, the renderer, the menus and `QOL.asi`. A game with someone who
  lacks the plugins is a normal game for them.
- **Pay when queued** (Gameplay tab, `[Rules] PayOnQueue`) is the one rule that
  changes what every machine simulates: an item is paid for when it is queued, and
  refunded on cancel. It is on by default. In an *Internet - Online* game each joiner
  reports whether it runs the rules plugin with the setting on, and the host decides:
  **if everyone has it, the game plays pay-on-queue; if anyone has not, everyone plays
  stock rules.** It never desynchronises a game, it just is not pay-on-queue. If you
  do not want it, switch the row off or leave the *Rules* part out. The row is locked
  during a network game, and a note says so when the plugin is not installed.
- Other rows marked *Must match* are the same: set them before the game is created.
- Do not edit `RTS_CFG.h`. Network games compare a checksum of it, and an edited file
  locks you out of games with players who have the stock one. Refit never edits it.

## Choosing a graphics backend

Display8 draws on **Vulkan, then Direct3D 12**, whichever starts first
(`Backend=auto`, the default). You can see which in `Display8.log`
(`stub: the backend is Vulkan`). This is not on the Refit screen: set it in
`RefitPlayer.ini` or `Display8.ini`:

    [Display8.Display8]
    Backend=d3d12

| `Backend=` | Does |
|---|---|
| `auto` | Vulkan, then Direct3D 12, then the stock renderer |
| `vulkan` | Vulkan, then the stock renderer |
| `d3d12` | Direct3D 12, then the stock renderer |
| `d3d12,vulkan` | the list in that order |
| `none` | the stock renderer at once |

A change applies at the next launch. A value of the old `d3d9` is skipped; Direct3D 9
is no longer used.

**When to use `none`.** `none` is "the game as it shipped, drawn by the system's own
Direct3D 8": Display8 gets out of the way and `Render3D.asi` and `UI.asi` stand down,
so the 3D view and the HUD are stock's. Use it to:

- see whether a problem is Refit's renderer or something else (a crash that stays under
  `none` is not the renderer's);
- play on a machine where Vulkan and Direct3D 12 both fail. You do not have to set it:
  when neither can start, Display8 takes this path by itself and logs
  `bail-out: <why>`. The game opens at stock quality instead of not at all;
- compare against the stock look.

`QOL.asi`, `QOLRules.asi` and `Online.asi` do not draw and keep working under `none`.
Display8 also takes the same path on an executable it does not recognise.

`Display8.ini` has a few more: `VSync=0` lifts the frame limit, `[Samplers]` sets
anisotropic filtering and LOD bias (the settings the old `dxvk.conf` held), and
`Validate=1` turns on the graphics API's checking layer when your system has one, which
you only want when asked to for a bug report.

## When something is wrong

### The logs

Every plugin writes a log, rewritten at each launch, in

- Windows: `%LOCALAPPDATA%\armada2-refit\logs\`
- Linux and macOS (under Wine): the same path inside the game's Wine prefix, that is
  `<prefix>/drive_c/users/<user>/AppData/Local/armada2-refit/logs/`

The first line of each log says where it is. If that folder cannot be written, the
plugins fall back to the game folder. The files are `Display8.log`, `Render3D.log`,
`UI.log`, `QOL.log`, `QOLRules.log` and `Online.log`. An uninstall leaves the folder.
A log beside `Armada2.exe` whose first line is not `log: ...` is from a version before
13.50.0 and says nothing of the current one; installing takes such logs out.

What to look for first:

- `engine: build armada2/gog-1.1` is a recognised executable. `engine: unknown build`
  means the plugins run only what they can verify, and for most that is nothing. This
  is what a different edition of the game looks like.
- `engine: <hook> stands down: ...` names a single feature that did not start because
  the game's code at that place is not stock's. The rest runs.
- `Display8.log`: `stub: the backend is ...` is the working case. `bail-out: ...` says
  why the stock renderer was used.
- `settings: RefitPlayer.ini [...] ... overrides ...` records which of your overlay
  values were read.

### The bug report

The graphical installer's **Report** (the header menu on Linux, a link on Windows and
macOS) collects what helps from a distance: which files are in the game folder
(identified by hash, never copied), the installed release, the `.ini` keys that differ
from it, your `RefitPlayer.ini` lines, and the logs cut down to their useful parts. It
removes your user name and IP addresses. *Copy* puts it on the clipboard as a code
block for a GitHub issue. It names the likeliest cause first, where it knows one (a
`d3d8.dll` that is not Display8, a launcher not loading the folder's DLLs, a file an
antivirus holds).

### Troubleshooting

| Symptom | Likely cause and what to do |
|---|---|
| The game starts but looks and behaves as stock; no Refit entry in Options | The plugins did not load. On Linux and macOS the Wine overrides must include `winmm=n,b;d3d8=n,b`: rerun the installer, or check Heroic's settings for the game. Windows needs none. A GOG Galaxy "repair" puts GOG's `d3d8.dll` back: install again |
| The 3D view is stock's, the HUD and menus are ours (or the reverse) | A part is switched off (Refit screen, Graphics or Interface tab), or its log line says why it stood down. Check `Render3D.log` or `UI.log` |
| Black screen, or the game closes at start | Set `Backend=none` to separate the renderer from the rest. If it starts, send `Display8.log` with the report. A very old GPU driver is a common cause |
| Black bars at the sides of the main menu | The widescreen backdrop is built from your game's own art and is not in a release. Black sides are the intended fallback ([`ui/MENUS.md`](ui/MENUS.md)) |
| A plugin is missing after install, or the installer cannot copy one | Antivirus. See the README section linked above |
| The Refit screen's row is dimmed with a note | That part is not running; the note says why. Your choice is kept |
| A change in Refit has no effect | Check its badge: *Next mission* and *Restart* rows apply later |
| Joiners cannot connect | Check `Online.log` on both sides: it says whether the direct path or the relay was used. A host with `Server=` emptied in `Online.ini` needs the address |
| The game feels different in a network game | *Pay when queued* is on for that game (see "Multiplayer") |
| A different edition of the game (Armada 1, or a mod with an executable of its own) | Not supported; the plugins run as stock on a build they do not know. Armada 1 is planned ([`PLAN-ARMADA1.md`](PLAN-ARMADA1.md)) |

When you open an issue, attach the installer's report, the version of the release
installed (the installer shows it), and say which backend `Display8.log` names.

## Taking it out

The installer's **Uninstall** (or `install.sh --uninstall`, `install.ps1 -Uninstall`)
removes everything that release installed, and nothing it did not put there. It:

- takes out the plugins and their `.ini` files from the game folder, with any log an
  earlier version wrote there;
- puts back GOG's `d3d8.dll` from `d3d8.dll.gog-backup` and the stock `binkw32.dll`;
- takes out the loader and widescreen patch only if it added them;
- on Linux and macOS, takes its changes back out of Heroic's settings or the bottle's
  registry, and only the values that are still as it left them.

The models it rewrote go back to stock. It leaves `RefitPlayer.ini` (so that a later
install finds your choices) and the logs folder; delete them by hand for a clean
slate. Textures and movies built with the repository's tools are not in a release and
are reverted by those tools (`./a2tex revert all`).

The switch the repository calls `./a2mod stock` / `refit`, which flips the whole game
for a before/after, is a tool of the repository checkout and is not in a release. With
a release installed, the equivalents are: the Refit screen for one part, `Backend=none`
for the whole renderer, and the installer's Uninstall for all of it.

## More

- The parts, one README each: [`ui/`](ui/README.md), [`render3d/`](render3d/README.md),
  [`qol/`](qol/README.md), [`rules/`](rules/README.md), [`online/`](online/README.md),
  [`display/`](display/README.md).
- Every change and whether it has been seen in game: each folder's `CHANGELOG.md`, and
  the root [`CHANGELOG.md`](CHANGELOG.md).
- What was removed and how it is cleaned up: [`retire/`](retire/README.md).
- Building your own mod: [`MODDING.md`](MODDING.md).
