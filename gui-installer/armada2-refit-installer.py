#!/usr/bin/env python3
"""Armada II Refit -- the graphical installer, for Linux, Windows and macOS (GOG release).

    python3 armada2-refit-installer.py            the window
    python3 armada2-refit-installer.py --list     what it finds: games, releases
    python3 armada2-refit-installer.py --install [VERSION] [--game DIR] [--no-launcher]
    python3 armada2-refit-installer.py --package ZIP --game DIR [--no-launcher]
    python3 armada2-refit-installer.py --uninstall [--game DIR]
    python3 armada2-refit-installer.py --selftest

It finds the game, lists the releases on GitHub -- checked on every launch and on
Refresh, cached so it works offline -- and installs the chosen one by running the
release's own installer script, following its progress: install.sh on Linux (Heroic,
Lutris, Bottles, Wine prefixes) and macOS (a Wine prefix: CrossOver, Sikarugir, Whisky,
plain Wine), install.ps1 on Windows (plain GOG). It writes the launch settings the game
needs where its launcher reads them: Heroic's config for the game on Linux, the Wine
prefix's registry on macOS; Windows needs none. Textures and the cutscene player are not
installed from here, for now.

It reads release packages of the schemas in SCHEMAS. The schema is the layout of the
release zip as this program relies on it (manifest.json, the scripts' ::step lines),
versioned on its own: gui-installer/README.md, "The package schema". Standalone: Python 3
and the standard library; the window is GTK 4 and libadwaita on Linux (the AppImage
carries them) and Tk on Windows and macOS (the .exe and the .app carry it).

Frozen into an .exe or an .app, the program also stands in for Python: given a .py file
as its first argument it runs it, so the release scripts' Python steps (the SOD
rewrites) need no Python on the player's machine (A2_PYTHON).
"""
import argparse    # noqa: F401  argparse, math and struct: for the SOD scripts the frozen
import fnmatch
import glob
import hashlib
import json
import math        # noqa: F401  program runs as Python (run_python), so they are bundled
import os
import re
import shutil
import sqlite3
import ssl
import struct      # noqa: F401
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
import zipfile

INSTALLER_VERSION = '1.5.0'
SCHEMAS = (0, 1)          # 0: the zips before manifest.json; 1: manifest.json, ::step lines
REPO = 'cedrichaase/armada2-refit'
API = f'https://api.github.com/repos/{REPO}/releases?per_page=30'
RELEASES_PAGE = f'https://github.com/{REPO}/releases'
SYSTEM = {'win32': 'windows', 'darwin': 'macos'}.get(sys.platform, 'linux')
# The installer as each system downloads it from a release.
INSTALLER_ASSETS = {'linux': 'armada2-refit-installer-x86_64.AppImage',
                    'windows': 'armada2-refit-installer-windows.exe',
                    'macos': 'armada2-refit-installer-macos.zip'}
INSTALLER_ASSET = INSTALLER_ASSETS[SYSTEM]
SYSTEM_NAME = {'linux': 'Linux', 'windows': 'Windows', 'macos': 'macOS'}[SYSTEM]
FROZEN = bool(getattr(sys, 'frozen', False))    # a PyInstaller .exe or .app
NO_WINDOW = 0x08000000 if SYSTEM == 'windows' else 0   # CREATE_NO_WINDOW: no console flashes
STATE_FILE = 'armada2-refit-installed.json'     # in the game directory: what we installed
GOG_APP = '1174788223'                          # Star Trek: Armada II on GOG
# Layers a package carries that this installer leaves out (install.sh's A2_SKIP): the
# cutscene player is not installed from here yet. Textures are not in a package at all.
EXCLUDED = ('cutscenes',)

HOME = os.path.expanduser('~')
XDG_CACHE = os.environ.get('XDG_CACHE_HOME') or os.path.join(HOME, '.cache')
XDG_CONFIG = os.environ.get('XDG_CONFIG_HOME') or os.path.join(HOME, '.config')
XDG_DATA = os.environ.get('XDG_DATA_HOME') or os.path.join(HOME, '.local/share')
if SYSTEM == 'windows':
    _local = os.environ.get('LOCALAPPDATA') or os.path.join(HOME, 'AppData', 'Local')
    CACHE = os.path.join(_local, 'armada2-refit', 'installer', 'cache')
    SETTINGS = os.path.join(os.environ.get('APPDATA') or os.path.join(HOME, 'AppData', 'Roaming'),
                            'armada2-refit', 'installer.json')
elif SYSTEM == 'macos':
    CACHE = os.path.join(HOME, 'Library', 'Caches', 'armada2-refit', 'installer')
    SETTINGS = os.path.join(HOME, 'Library', 'Application Support', 'armada2-refit',
                            'installer.json')
else:
    CACHE = os.path.join(XDG_CACHE, 'armada2-refit', 'installer')
    SETTINGS = os.path.join(XDG_CONFIG, 'armada2-refit', 'installer.json')


# The install.sh stages, in order, and what the window says during each.
STEPS = ['verify', 'prereqs', 'chain', 'hud', 'menus', 'qol', 'grid', 'models', 'lighting', 'sky',
         'online', 'msaa', 'cutscenes', 'renderer', 'bloom', 'done']
STATUS = {
    'fetch': 'Checking for releases',
    'download': 'Downloading release {version}',
    'unpack': 'Unpacking the package',
    'verify': 'Verifying the package',
    'prereqs': 'Installing the widescreen patch and plugin loader',
    'hud': 'Installing the HUD',
    'menus': 'Installing the menus',
    'qol': 'Installing quality of life',
    'lighting': 'Installing lighting',
    'sky': 'Installing the sky',
    'online': 'Installing online play',
    'msaa': 'Installing anti-aliasing',
    'cutscenes': 'Skipping the cutscene player',
    'renderer': 'Configuring the renderer',
    'bloom': 'Setting up bloom',
    'done': 'Finishing up',
    'launcher': 'Writing the launch settings',
    'chain': 'Setting up the Direct3D chain',
    'grid': 'Installing the grid keys',
    'models': 'Rewriting the models',
    'uninstall': 'Removing the mod',
    'restore': 'Restoring the launch settings',
}
# Zips of schema 0 print no ::step lines; their install.sh output marks the stages.
LEGACY_MARKS = [('prerequisites:', 'prereqs'), ('  HUD.asi', 'hud'), ('  Menus.asi', 'menus'),
                ('  QOL.asi', 'qol'), ('  Lighting.asi', 'lighting'), ('  Online.asi', 'online'),
                ('MSAA', 'msaa'),
                ('  binkw32.dll', 'cutscenes'), ('dxvk.conf', 'renderer'), ('bloom', 'bloom'),
                ('In the launcher', 'done')]
# ...and their README.txt names the layers; this fills in the rest.
LEGACY_LAYERS = {
    'HUD.asi': ('hud', 'HUD', 'always'), 'Menus.asi': ('menus', 'Menus', 'always'),
    'QOL.asi': ('qol', 'Quality of life', 'always'),
    'Lighting.asi': ('lighting', 'Lighting', 'always'),
    'Online.asi': ('online', 'Online', 'always'),
    'MSAA.asi': ('msaa', 'Anti-aliasing', 'dxvk'),
    'binkw32.dll': ('cutscenes', 'Cutscene player', 'always'),
    'dxvk.conf': ('renderer', 'Renderer', 'dxvk.conf'),
}

def vtuple(v):
    return tuple(int(x) for x in re.findall(r'\d+', v)) or (0,)


def human(n):
    for unit in ('B', 'KB', 'MB', 'GB'):
        if n < 1024 or unit == 'GB':
            return f'{n:.0f} {unit}' if unit == 'B' else f'{n:.1f} {unit}'
        n /= 1024


def short(path):
    return '~' + path[len(HOME):] if path.startswith(HOME + os.sep) else path


def sha256(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def load_json(path, default=None):
    try:
        with open(path) as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def save_json(path, data, indent=1):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + '.tmp'
    with open(tmp, 'w') as f:
        json.dump(data, f, indent=indent)
        f.write('\n')
    os.replace(tmp, path)


def processes():
    """(comm, cmdline) of every process we can see: /proc on Linux, tasklist on Windows
    (image names only), ps on macOS."""
    out = []
    if SYSTEM == 'windows':
        try:
            r = subprocess.run(['tasklist', '/fo', 'csv', '/nh'], capture_output=True, text=True,
                               errors='replace', creationflags=NO_WINDOW, timeout=10)
        except (OSError, subprocess.SubprocessError):
            return out
        for line in r.stdout.splitlines():
            name = line.split('","')[0].strip('"')
            if name:
                out.append((name, name))
        return out
    if SYSTEM == 'macos':
        try:
            r = subprocess.run(['ps', '-axo', 'comm=,args='], capture_output=True, text=True,
                               errors='replace', timeout=10)
        except (OSError, subprocess.SubprocessError):
            return out
        for line in r.stdout.splitlines():
            comm, _, cmd = line.strip().partition(' ')
            out.append((os.path.basename(comm), line.strip()))
        return out
    for pid in os.listdir('/proc'):
        if not pid.isdigit():
            continue
        try:
            with open(f'/proc/{pid}/comm') as f:
                comm = f.read().strip()
            with open(f'/proc/{pid}/cmdline', 'rb') as f:
                cmd = f.read().replace(b'\0', b' ').decode(errors='replace')
        except OSError:
            continue
        out.append((comm, cmd))
    return out


def game_running():
    return any('armada2.exe' in cmd.lower() for _, cmd in processes())


def heroic_running():
    return any(comm.lower().startswith('heroic') for comm, _ in processes())


# ------------------------------------------------------------------ finding the game

class Heroic:
    """Heroic's record of the GOG game: where its per-game config lives."""
    def __init__(self, root, app, flatpak):
        self.root, self.app, self.flatpak = root, app, flatpak
        self.config = os.path.join(root, 'GamesConfig', f'{app}.json')

    def label(self):
        return 'Heroic (Flatpak)' if self.flatpak else 'Heroic'


class Game:
    """An install of the game. `heroic` (Linux) or `prefix` (macOS) is what launches it
    and where its launch settings go; on Windows neither is needed."""
    def __init__(self, path, source, heroic=None, prefix=None):
        self.path, self.source, self.heroic, self.prefix = path, source, heroic, prefix

    def label(self):
        return f'{self.source} — {os.path.basename(self.path)}'


def exe_in(d):
    try:
        return any(n.lower() == 'armada2.exe' for n in os.listdir(d))
    except OSError:
        return False


HEROIC_ROOTS = [(os.path.join(XDG_CONFIG, 'heroic'), False),
                (os.path.join(HOME, '.var/app/com.heroicgameslauncher.hgl/config/heroic'), True)]


def heroic_entries():
    """(install path, Heroic) for every GOG game Heroic has installed."""
    out = []
    for root, flatpak in HEROIC_ROOTS:
        data = load_json(os.path.join(root, 'gog_store', 'installed.json'), {})
        for g in (data or {}).get('installed', []):
            p = g.get('install_path')
            if p and g.get('appName'):
                out.append((p, Heroic(root, str(g['appName']), flatpak)))
    return out


def a2env_game():
    """A2_GAME as the repository's a2env resolves it: the environment, then the config."""
    g = os.environ.get('A2_GAME') or os.environ.get('A2_GAME_DIR') or os.environ.get('A2_DIR')
    if g:
        return g
    conf = os.environ.get('A2_CONF') or os.path.join(XDG_CONFIG, 'armada2-refit.conf')
    try:
        for line in open(conf):
            k, sep, v = line.strip().partition('=')
            if sep and k == 'A2_GAME':
                return os.path.expanduser(v.replace('$HOME', HOME))
    except OSError:
        pass
    return None


def lutris_dirs():
    out = []
    for db in (os.path.join(XDG_DATA, 'lutris/pga.db'),
               os.path.join(HOME, '.var/app/net.lutris.Lutris/data/lutris/pga.db')):
        if not os.path.exists(db):
            continue
        try:
            con = sqlite3.connect(f'file:{db}?mode=ro', uri=True)
            rows = con.execute('select name, directory from games').fetchall()
            con.close()
        except sqlite3.Error:
            continue
        out += [d for name, d in rows if d and 'armada' in (name or '').lower()]
    return out


def prefixes():
    """Wine prefixes worth a look: the default one, Heroic's, Lutris' and Bottles'."""
    pats = ['~/.wine', '~/Games/*', '~/Games/Heroic/Prefixes/*', '~/.local/share/wineprefixes/*',
            '~/.local/share/bottles/bottles/*',
            '~/.var/app/com.usebottles.bottles/data/bottles/bottles/*',
            '~/.var/app/com.heroicgameslauncher.hgl/data/heroic/prefixes/*']
    for pat in pats:
        for p in glob.glob(os.path.expanduser(pat)):
            for sub in ('', 'pfx'):
                c = os.path.join(p, sub, 'drive_c')
                if os.path.isdir(c):
                    yield c


def find_in(root, depth):
    """Folders holding Armada2.exe at most `depth` levels below root, whose path names
    Armada (the game's folder always does), so the walk stays small."""
    found = []
    base = root.rstrip(os.sep).count(os.sep)
    for d, subs, _ in os.walk(root):
        level = d.count(os.sep) - base
        if exe_in(d):
            found.append(d)
            subs[:] = []
            continue
        if level >= depth:
            subs[:] = []
            continue
        # Below the first level only folders that may lead to the game.
        keep = ('armada', 'gog', 'program files', 'games', 'activision', 'star trek')
        subs[:] = [s for s in subs if level == 0 or any(k in s.lower() for k in keep)]
    return found


def find_games():
    """Every Armada II install found, best first, without duplicates."""
    found = []

    def add(path, source, heroic=None, prefix=None):
        if not path or not exe_in(path):
            return
        real = os.path.realpath(path)
        for g in found:
            if os.path.realpath(g.path) == real:
                g.heroic = g.heroic or heroic
                g.prefix = g.prefix or prefix
                return
        found.append(Game(real, source, heroic, prefix or
                          (prefix_of(real) if SYSTEM == 'macos' else None)))

    if SYSTEM == 'windows':
        for p in gog_registry_dirs():
            add(p, 'GOG')
        add(a2env_game(), 'A2_GAME')
        for root in windows_roots():
            for p in find_in(root, 3):
                add(p, 'Folder')
        return found
    if SYSTEM == 'macos':
        for pre in mac_prefixes():
            for p in find_in(os.path.join(pre.root, 'drive_c'), 4):
                add(p, pre.label(), prefix=pre)
        add(a2env_game(), 'A2_GAME')
        return found

    heroic = heroic_entries()
    for p, h in heroic:
        if h.app == GOG_APP or 'armada' in p.lower():
            add(p, f'{h.label()} · GOG', h)
    add(a2env_game(), 'A2_GAME')
    for d in lutris_dirs():
        for p in [d] + find_in(d, 5):
            add(p, 'Lutris')
    for c in prefixes():
        for p in find_in(c, 4):
            add(p, 'Wine prefix')
    for pat in ('~/Games', '~/GOG Games', '~/games'):
        root = os.path.expanduser(pat)
        if os.path.isdir(root):
            for p in find_in(root, 3):
                add(p, 'Folder')
    # Any install Heroic knows, whichever way it was found, gets its launcher settings.
    for g in found:
        for p, h in heroic:
            if os.path.realpath(p) == os.path.realpath(g.path):
                g.heroic = g.heroic or h
    return found


# ------------------------------------------------------------------ Windows: GOG

def gog_registry_dirs():
    """Where the registry says GOG put the game: GOG Galaxy and GOG's offline installer
    both write HKLM\\SOFTWARE\\[WOW6432Node\\]GOG.com\\Games\\<id>, with its folder in `path`."""
    if SYSTEM != 'windows':
        return []
    import winreg
    out = []
    for base in (r'SOFTWARE\WOW6432Node\GOG.com\Games', r'SOFTWARE\GOG.com\Games'):
        try:
            key = winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, base)
        except OSError:
            continue
        with key:
            i = 0
            while True:
                try:
                    sub = winreg.EnumKey(key, i)
                except OSError:
                    break
                i += 1
                vals = {}
                try:
                    with winreg.OpenKey(key, sub) as g:
                        for n in ('gameName', 'path'):
                            try:
                                vals[n] = str(winreg.QueryValueEx(g, n)[0])
                            except OSError:
                                vals[n] = ''
                except OSError:
                    continue
                if vals['path'] and (sub == GOG_APP or 'armada' in vals['gameName'].lower()):
                    out.append(vals['path'])
    return out


def windows_roots():
    """The folders GOG installs into by default, on every fixed drive."""
    out = []
    for d in 'CDEFGHIJ':
        drive = f'{d}:\\'
        if not os.path.isdir(drive):
            continue
        for sub in ('GOG Games', 'Games', r'Program Files (x86)\GOG Galaxy\Games',
                    r'Program Files (x86)\GOG.com', r'Program Files (x86)', 'Program Files'):
            if os.path.isdir(os.path.join(drive, sub)):
                out.append(os.path.join(drive, sub))
    return out


# ------------------------------------------------------------------ macOS: Wine prefixes

class WinePrefix:
    """A Wine prefix holding the game, on macOS: a CrossOver bottle, a Whisky bottle, a
    Sikarugir / Kegworks / Wineskin wrapper (an .app with the prefix inside it), or a
    plain one. The DLL overrides go into its registry (user.reg), and prefix_wine() is
    how the game starts in it."""
    def __init__(self, root, kind, name=None, bundle=None):
        self.root, self.kind, self.bundle = root, kind, bundle
        base = os.path.basename(bundle or root)
        self.name = name or (base[:-4] if base.endswith('.app') else base)
        self.userreg = os.path.join(root, 'user.reg')

    def label(self):
        return f'Wine · {short(self.root)}' if self.kind == 'Wine' else f'{self.kind} · {self.name}'


CROSSOVER_BOTTLES = '~/Library/Application Support/CrossOver/Bottles'
WHISKY_BOTTLES = '~/Library/Containers/com.isaacmarovitz.Whisky/Bottles'
WRAPPER_DIRS = ('Sikarugir', 'Kegworks', 'Wineskin', 'Porting Kit', '')


def mac_prefixes():
    """Every Wine prefix worth a look, CrossOver's first."""
    out = []
    for kind, where in (('CrossOver', CROSSOVER_BOTTLES), ('Whisky', WHISKY_BOTTLES)):
        for b in sorted(glob.glob(os.path.join(os.path.expanduser(where), '*'))):
            if os.path.isdir(os.path.join(b, 'drive_c')):
                out.append(WinePrefix(b, kind))
    for apps in (os.path.expanduser('~/Applications'), '/Applications'):
        for sub in WRAPPER_DIRS:
            for app in sorted(glob.glob(os.path.join(apps, sub, '*.app'))):
                pre = os.path.join(app, 'Contents', 'SharedSupport', 'prefix')
                if os.path.isdir(os.path.join(pre, 'drive_c')):
                    out.append(WinePrefix(pre, 'Wrapper', bundle=app))
    for pat in ('~/.wine', '~/Wine/*', '~/Games/*', '~/.local/share/wineprefixes/*'):
        for p in sorted(glob.glob(os.path.expanduser(pat))):
            if os.path.isdir(os.path.join(p, 'drive_c')):
                out.append(WinePrefix(p, 'Wine'))
    return out


def prefix_of(path):
    """The WinePrefix a game folder is in: the folder above its drive_c."""
    d = os.path.realpath(path)
    while True:
        parent = os.path.dirname(d)
        if parent == d:
            return None
        if os.path.basename(d) == 'drive_c':
            for pre in mac_prefixes():
                if os.path.realpath(pre.root) == parent:
                    return pre
            return WinePrefix(parent, 'Wine')
        d = parent


CROSSOVER_APP = '/Applications/CrossOver.app'
MAC_WINES = ('/opt/homebrew/bin/wine', '/usr/local/bin/wine',
             '/Applications/Wine Stable.app/Contents/Resources/wine/bin/wine',
             '/Applications/Wine Devel.app/Contents/Resources/wine/bin/wine',
             '/Applications/Wine Staging.app/Contents/Resources/wine/bin/wine')


def prefix_wine(pre):
    """(argv before the exe, extra environment) to start a program in the prefix, or
    None when no Wine for it is found."""
    if pre.kind == 'CrossOver':
        for app in (CROSSOVER_APP, os.path.expanduser('~' + CROSSOVER_APP)):
            w = os.path.join(app, 'Contents/SharedSupport/CrossOver/bin/wine')
            if os.path.isfile(w):
                return [w, '--bottle', pre.name], {}
        return None
    if pre.bundle:
        for w in ('Contents/SharedSupport/wine/bin/wine', 'Contents/SharedSupport/wine/bin/wine64',
                  'Contents/Resources/wine/bin/wine'):
            if os.path.isfile(os.path.join(pre.bundle, w)):
                return [os.path.join(pre.bundle, w)], {'WINEPREFIX': pre.root}
    if pre.kind == 'Whisky':
        w = os.path.expanduser('~/Library/Application Support/com.isaacmarovitz.Whisky/'
                               'Libraries/Wine/bin/wine64')
        if os.path.isfile(w):
            return [w], {'WINEPREFIX': pre.root}
    for w in (shutil.which('wine'),) + MAC_WINES:
        if w and os.path.isfile(w):
            return [w], {'WINEPREFIX': pre.root}
    return None


# The overrides go under AppDefaults\Armada2.exe, so they apply to the game alone and
# not to everything else in the bottle. user.reg spells a key with doubled backslashes.
REG_KEY = r'Software\\Wine\\AppDefaults\\Armada2.exe\\DllOverrides'
REG_MODES = {'n': 'native', 'b': 'builtin'}
REG_VALUE = re.compile(r'^"\*?([^"]+)"="([^"]*)"\s*$')


def reg_mode(m):
    return ','.join(REG_MODES.get(x.strip(), x.strip()) for x in m.split(','))


def reg_section(lines, key=REG_KEY):
    """(header index, end index) of `key` among user.reg's lines, or None."""
    want = '[' + key.lower() + ']'
    for i, l in enumerate(lines):
        if l.startswith('[') and l.lower().split(']')[0] + ']' == want:
            j = i + 1
            while j < len(lines) and not lines[j].startswith('['):
                j += 1
            while j > i + 1 and not lines[j - 1].strip():
                j -= 1          # the blank line before the next section stays where it is
            return i, j
    return None


def reg_values(lines, sec):
    """{dll: value} in a DllOverrides section; Wine's '*' prefix is the same DLL."""
    out = {}
    for l in lines[sec[0] + 1:sec[1]]:
        m = REG_VALUE.match(l)
        if m:
            out[m.group(1).lower()] = m.group(2)
    return out


def read_reg(path):
    with open(path, encoding='utf-8', errors='surrogateescape', newline='') as f:
        return f.read().split('\n')


def wineserver_running():
    return any(comm.lower().startswith('wineserver') for comm, _ in processes())


def prefix_plan(pre, wanted):
    """(changes, conflicts) to give the game `wanted` in the prefix's registry. Only DLL
    overrides can go there; one the game has already is kept, as in Heroic's."""
    if not os.path.isfile(pre.userreg):
        return None, [f'{pre.label()} has no registry yet: start any program in it once, '
                      'then install again']
    lines = read_reg(pre.userreg)
    sec = reg_section(lines)
    have = reg_values(lines, sec) if sec else {}
    changes, conflicts = [], []
    for k, v in wanted.items():
        if k != 'WINEDLLOVERRIDES':
            conflicts.append(f'{k}={v} has to be set in the launcher by hand')
            continue
        for dll, mode in split_overrides(v).items():
            if dll.lower() not in have:
                changes.append(dict(key=dll, old=None, new=reg_mode(mode)))
    return changes, conflicts


def prefix_apply(pre, changes, undo=False):
    """Write `changes` into the prefix's user.reg, or take them back out (only values
    still as we left them). Wine must not be running in the prefix, since wineserver
    writes its own copy back on exit: the caller checks. The first write keeps the
    original as user.reg.a2refit-backup."""
    if not changes or not os.path.isfile(pre.userreg):
        return []
    lines = read_reg(pre.userreg)
    backup = pre.userreg + '.a2refit-backup'
    if not undo and not os.path.exists(backup):
        shutil.copy2(pre.userreg, backup)
    sec = reg_section(lines)
    done = []
    if not undo:
        if sec is None:
            while lines and lines[-1] == '':
                lines.pop()
            lines += ['', f'[{REG_KEY}] {int(time.time())}']
            sec = (len(lines) - 1, len(lines))
        lines[sec[1]:sec[1]] = [f'"{c["key"]}"="{c["new"]}"' for c in changes]
        done = list(changes)
    elif sec is not None:
        keep = []
        for l in lines[sec[0] + 1:sec[1]]:
            m = REG_VALUE.match(l)
            c = next((c for c in changes if m and m.group(1).lower() == c['key'].lower()
                      and m.group(2) == c['new']), None)
            if c:
                done.append(c)
            else:
                keep.append(l)
        if any(REG_VALUE.match(l) for l in keep):
            lines[sec[0] + 1:sec[1]] = keep
        else:                   # the section held only ours: it goes, with its blank line
            lo = sec[0] - 1 if sec[0] > 0 and not lines[sec[0] - 1].strip() else sec[0]
            lines[lo:sec[1]] = []
    while lines and lines[-1] == '':
        lines.pop()
    lines.append('')
    tmp = pre.userreg + '.tmp'
    with open(tmp, 'w', encoding='utf-8', errors='surrogateescape', newline='') as f:
        f.write('\n'.join(lines))
    shutil.copymode(pre.userreg, tmp)
    os.replace(tmp, pre.userreg)
    return done


def launch_game_native(game, env_add=None):
    """Start the game without Heroic: the exe itself on Windows, through the prefix's
    Wine on macOS (with the launch variables in its environment too, should the
    registry not have them). False when there is no way to."""
    try:
        exe = next((os.path.join(game.path, n) for n in os.listdir(game.path)
                    if n.lower() == 'armada2.exe'), None)
    except OSError:
        exe = None
    if not exe:
        return False
    try:
        if SYSTEM == 'windows':
            subprocess.Popen([exe], cwd=game.path, close_fds=True,
                             creationflags=0x00000008 | 0x00000200)   # DETACHED, NEW_GROUP
            return True
        w = prefix_wine(game.prefix) if game.prefix else None
        if not w:
            return False
        argv, extra = w
        env = dict(host_env(), **extra, **(env_add or {}))
        subprocess.Popen(argv + [exe], cwd=game.path, env=env, stdout=subprocess.DEVNULL,
                         stderr=subprocess.DEVNULL, start_new_session=True)
    except OSError:
        return False
    return True


# ------------------------------------------------------------------ the game's state

def is_dxvk(path):
    try:
        with open(path, 'rb') as f:
            return b'dxvk' in f.read().lower()
    except OSError:
        return False


D3D8TO9_SHA = '122928cfe225c25d30decf7184a5d37e490cecf3b58256ba3206c7e1853f8ab8'
GOG_D3D8_SHA = '735dbb81a5fa0368c6436349fc5bf97a9ae15daeb6829dfe95a17704294e6719'


def inspect(game):
    """What install.sh will find in the game directory -- the same tests it makes."""
    p = lambda n: os.path.join(game, n)
    d3d8to9 = os.path.isfile(p('d3d8.dll')) and sha256(p('d3d8.dll')) == D3D8TO9_SHA
    dxvk = is_dxvk(p('d3d8.dll')) or (d3d8to9 and is_dxvk(p('d3d9.dll')))
    layers = [os.path.join(XDG_DATA, 'vulkan/implicit_layer.d'), '/etc/vulkan/implicit_layer.d',
              '/usr/share/vulkan/implicit_layer.d']
    vkbasalt = SYSTEM == 'linux' and \
        any(glob.glob(os.path.join(d, '*[Bb]asalt*.json')) for d in layers)
    conf = p('dxvk.conf')
    foreign_conf = os.path.isfile(conf) and \
        'generated by tools/renderer-config.sh' not in open(conf, errors='replace').read()
    state = load_json(p(STATE_FILE))
    installed = state.get('version') if isinstance(state, dict) else None
    if not installed and os.path.exists(p('HUD.asi')):
        installed = '?'
    # What the scripts' chain step will leave (install.sh, install.ps1): crosire's d3d8to9
    # goes in over no d3d8.dll or GOG's, and over DXVK's except on Windows, where a DXVK
    # d3d8.dll was put there by hand and stays; on Linux not past a d3d9.dll that is not
    # DXVK's, and with DXVK's d3d9 put in behind it. On a Mac, never DXVK's d3d9.
    d8, d9 = p('d3d8.dll'), p('d3d9.dll')
    d8_ok = d3d8to9 or not os.path.exists(d8) or \
        (os.path.isfile(d8) and sha256(d8) == GOG_D3D8_SHA) or \
        (is_dxvk(d8) and SYSTEM != 'windows')
    if SYSTEM == 'linux' and os.path.exists(d9) and not is_dxvk(d9):
        d8_ok = False
    after_d3d8to9 = d8_ok or d3d8to9
    after_dxvk = dxvk if not d8_ok else \
        (SYSTEM == 'linux' or (SYSTEM == 'windows' and is_dxvk(d9)))
    return dict(dxvk=dxvk, d3d8to9=d3d8to9, vkbasalt=vkbasalt, foreign_conf=foreign_conf,
                after_d3d8to9=after_d3d8to9, after_dxvk=after_dxvk,
                prereqs=os.path.exists(p('STA2WidescreenPatch.asi')),
                reshade=os.path.exists(p('ReShade.ini')),
                d3d9=os.path.exists(p('d3d9.dll')), installed=installed,
                state=state if isinstance(state, dict) else None)


def layer_verdict(layer, facts):
    """(will it go in, why) for one manifest layer on this game."""
    when = layer.get('when', 'always')
    if layer['id'] in EXCLUDED:
        return None, 'not available yet'
    if when == 'missing' and facts['prereqs']:
        return True, 'already there'
    if layer['id'] == 'msaa' and SYSTEM == 'macos':
        return False, 'not on macOS — needs DXVK, which Direct3D 9 on a Mac cannot use'
    if when == 'dxvk' and not facts.get('after_dxvk', facts['dxvk']):
        return False, 'skipped — needs DXVK in the game folder'
    if when == 'dxvk.conf' and SYSTEM == 'macos':
        return False, 'not on macOS — only DXVK reads it'
    if when == 'dxvk.conf' and facts['foreign_conf']:
        return False, 'skipped — your own dxvk.conf stays'
    if when == 'vkbasalt' and SYSTEM == 'windows' and not facts.get('reshade'):
        return False, 'skipped — ReShade is not installed'
    if when == 'vkbasalt' and SYSTEM == 'windows':
        return True, 'will install · ReShade preset'
    if when == 'vkbasalt' and SYSTEM == 'macos':
        return False, 'not on macOS yet'
    if when == 'vkbasalt' and not facts['vkbasalt']:
        return False, 'skipped — no vkBasalt layer'
    d3d8to9 = facts.get('after_d3d8to9', facts['d3d8to9'])
    if layer['id'] == 'lighting' and not d3d8to9:
        return True, 'will install · per vertex'
    if layer['id'] == 'sky' and not d3d8to9:
        return True, 'will install · stock sky without d3d8to9'
    return True, 'will install'


def launch_env(facts, package=None):
    """The launcher variables install.sh will print for this game; none on Windows,
    where the game folder's DLLs load first anyway."""
    if SYSTEM == 'windows':
        return {}
    env = {'WINEDLLOVERRIDES': 'winmm=n,b;d3d8=n,b' + (';d3d9=n,b' if facts['d3d9'] else '')}
    has_bloom = package is None or any(l['id'] == 'bloom' for l in package.layers)
    if facts['vkbasalt'] and has_bloom:
        bloom = os.path.join(XDG_DATA, 'armada2-refit-bloom')
        env['ENABLE_VKBASALT'] = '1'
        env['VKBASALT_CONFIG_FILE'] = os.path.join(bloom, 'vkBasalt.conf')
    return env


def unproxy(game, log=print):
    """The cutscene player out, the stock binkw32.dll back -- as install.sh's unproxy."""
    p = lambda n: os.path.join(game, n)
    try:
        with open(p('binkw32.dll'), 'rb') as f:
            proxy = b'BinkProxy' in f.read()
    except OSError:
        proxy = False
    if proxy:
        if os.path.isfile(p('binkw32.dll.a2neb-backup')):
            os.replace(p('binkw32.dll.a2neb-backup'), p('binkw32.dll'))
            if os.path.exists(p('binkw32_orig.dll')):
                os.unlink(p('binkw32_orig.dll'))
        elif os.path.isfile(p('binkw32_orig.dll')):
            os.replace(p('binkw32_orig.dll'), p('binkw32.dll'))
        else:
            log('binkw32.dll is the proxy and there is no stock copy -- restore it by hand')
            return
        log('  cutscene player left out: stock binkw32.dll restored')
    for n in ('BinkProxy.ini', 'BinkProxy.log'):
        if os.path.exists(p(n)):
            os.unlink(p(n))


# ------------------------------------------------------------------ vkBasalt, per distro

REPO_GIT = f'https://github.com/{REPO}.git'
# Ubuntu builds vkbasalt for every architecture but i386, so it is built from source
# with the project's own script, into the home folder. The same build is the fallback
# anywhere else.
BUILD_VKBASALT = (f'git clone --depth 1 {REPO_GIT} ~/armada2-refit && '
                  '~/armada2-refit/postfx/vkbasalt/build.sh')
# What to install where, by distribution family. Players paste these as they stand, so
# keep them runnable as written.
VKBASALT_HOWTO = {
    'arch': ('Arch Linux', [
        ('vkBasalt is in the AUR. With the [multilib] repository enabled and an AUR helper:',
         'yay -S lib32-vkbasalt')]),
    'fedora': ('Fedora', [
        ("The 32-bit package is in Fedora's own repositories:",
         'sudo dnf install vkBasalt.i686')]),
    'fedora-ostree': ('Fedora Atomic (Silverblue, Kinoite, ...)', [
        ('Layer the 32-bit package, then restart:',
         'sudo rpm-ostree install vkBasalt.i686')]),
    'debian': ('Debian', [
        ('Debian packages the 32-bit layer; enable i386 packages and install it:',
         'sudo dpkg --add-architecture i386 && sudo apt update && sudo apt install vkbasalt:i386')]),
    'ubuntu': ('Ubuntu', [
        ('Ubuntu has no 32-bit vkBasalt package, so build it into your home folder. '
         'First what the build needs:',
         'sudo dpkg --add-architecture i386 && sudo apt update && sudo apt install git file '
         'gcc-multilib g++-multilib glslang-tools pkg-config python3-venv libx11-dev:i386'),
        ('then the build (about a minute; installs into ~/.local only):', BUILD_VKBASALT)]),
    'other': ('Other distributions', [
        ("Install your distribution's 32-bit vkBasalt (named like vkbasalt:i386, "
         'vkBasalt.i686 or lib32-vkbasalt), or build it into your home folder — it needs '
         'gcc with 32-bit support, glslang, pkg-config, python3-venv and 32-bit libX11 '
         'headers:', BUILD_VKBASALT)]),
}


def os_release():
    out = {}
    for path in ('/etc/os-release', '/usr/lib/os-release'):
        try:
            for line in open(path):
                k, sep, v = line.strip().partition('=')
                if sep:
                    out[k] = v.strip().strip('"\'')
            return out
        except OSError:
            continue
    return out


def distro_family(osr=None):
    """'arch', 'fedora', 'fedora-ostree', 'debian', 'ubuntu' or 'other': the ID first,
    then ID_LIKE in order, so Mint and Pop!_OS are Ubuntu, CachyOS and Manjaro Arch."""
    osr = os_release() if osr is None else osr
    for t in [osr.get('ID', '')] + osr.get('ID_LIKE', '').split():
        if t in ('arch', 'archlinux'):
            return 'arch'
        if t == 'fedora':
            ostree = osr.get('VARIANT_ID') in ('silverblue', 'kinoite', 'sericea', 'onyx',
                                               'cosmic-atomic') or \
                os.path.exists('/run/ostree-booted')
            return 'fedora-ostree' if ostree else 'fedora'
        if t in ('ubuntu', 'debian'):
            return t
    return 'other'


# ------------------------------------------------------------------ Heroic, when missing

HEROIC_SITE = 'https://heroicgameslauncher.com/downloads'
HEROIC_FLATPAK = 'flatpak install flathub com.heroicgameslauncher.hgl'
# The native package first: a Flatpak Heroic cannot see a vkBasalt installed on the host.
HEROIC_HOWTO = {
    'arch': [('Heroic is in the AUR. With an AUR helper:', 'yay -S heroic-games-launcher-bin')],
    'fedora': [('Download the .rpm from heroicgameslauncher.com and install it, or use '
                'Flathub (bloom does not work in the Flatpak):', HEROIC_FLATPAK)],
    'debian': [('Download the .deb from heroicgameslauncher.com and install it, or use '
                'Flathub (bloom does not work in the Flatpak):', HEROIC_FLATPAK)],
    'other': [('Download Heroic from heroicgameslauncher.com, or use Flathub (bloom does '
               'not work in the Flatpak):', HEROIC_FLATPAK)],
}
HEROIC_HOWTO['ubuntu'] = HEROIC_HOWTO['debian']
HEROIC_HOWTO['fedora-ostree'] = [('Install Heroic from Flathub:', HEROIC_FLATPAK)]


def heroic_launch_uri(heroic):
    """What Heroic's own desktop shortcuts open (Heroic 2.x)."""
    return f'heroic://launch?appName={heroic.app}&runner=gog'


def launch_game(heroic):
    """Start the game through Heroic. xdg-open with the host's environment, so Heroic
    never inherits the AppImage's libraries; False when there is no xdg-open."""
    try:
        subprocess.Popen(['xdg-open', heroic_launch_uri(heroic)], env=host_env(),
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                         start_new_session=True)
    except OSError:
        return False
    return True


def heroic_installed():
    """Heroic on this machine: run once (its config folder), or installed and never run."""
    if any(os.path.isdir(root) for root, _ in HEROIC_ROOTS):
        return True
    if shutil.which('heroic', path=host_env().get('PATH')):
        return True
    return any(os.path.isdir(os.path.join(d, 'app/com.heroicgameslauncher.hgl'))
               for d in ('/var/lib/flatpak', os.path.join(XDG_DATA, 'flatpak')))


def vkbasalt_howto(family=None):
    """(what we detected, [(text, command)])."""
    osr = os_release()
    explicit = bool(family)
    family = family or distro_family(osr)
    name, steps = VKBASALT_HOWTO.get(family, VKBASALT_HOWTO['other'])
    pretty = osr.get('PRETTY_NAME') if not explicit else None
    return (pretty or name), steps


# ------------------------------------------------------------------ Heroic's settings

def split_overrides(v):
    out = {}
    for part in filter(None, (x.strip() for x in v.split(';'))):
        k, _, val = part.partition('=')
        out[k.strip()] = val.strip()
    return out


def heroic_plan(heroic, wanted):
    """(changes, conflicts) to give the game `wanted` in Heroic's config. A DLL override
    already there is kept and the missing ones added; any other variable already set to
    something else is left alone and reported."""
    data = load_json(heroic.config)
    if not isinstance(data, dict) or not isinstance(data.get(heroic.app), dict):
        return None, ['Heroic has no settings file for the game yet: open its settings '
                      'in Heroic once, then install again']
    env = {e.get('key'): e.get('value', '') for e in
           data[heroic.app].get('enviromentOptions', []) if isinstance(e, dict)}
    changes, conflicts = [], []
    for k, v in wanted.items():
        old = env.get(k)
        if k == 'WINEDLLOVERRIDES' and old:
            have = split_overrides(old)
            add = [f'{d}={m}' for d, m in split_overrides(v).items() if d not in have]
            if add:
                changes.append(dict(key=k, old=old, new=';'.join([old.rstrip(';')] + add)))
        elif old is None:
            changes.append(dict(key=k, old=None, new=v))
        elif old != v:
            conflicts.append(f'{k} is already set to {old} — left as it is')
    return changes, conflicts


def heroic_apply(heroic, changes, undo=False):
    """Write `changes` into Heroic's config (or take them back out). The first write
    keeps the original as <file>.a2refit-backup. Returns what was done."""
    if not changes:
        return []
    data = load_json(heroic.config)
    cfg = data[heroic.app]
    opts = cfg.setdefault('enviromentOptions', [])
    backup = heroic.config + '.a2refit-backup'
    if not os.path.exists(backup):
        shutil.copy2(heroic.config, backup)
    done = []
    for c in changes:
        entry = next((e for e in opts if isinstance(e, dict) and e.get('key') == c['key']), None)
        if not undo:
            if entry is None:
                opts.append({'key': c['key'], 'value': c['new']})
            else:
                entry['value'] = c['new']
            done.append(c)
        elif entry is not None and entry.get('value') == c['new']:
            # Only what is still as we left it goes back.
            if c['old'] is None:
                opts.remove(entry)
            else:
                entry['value'] = c['old']
            done.append(c)
    save_json(heroic.config, data, indent=2)
    return done


# ------------------------------------------------------------------ releases

class Release:
    def __init__(self, version, **kw):
        self.version = version
        self.prerelease = kw.get('prerelease', False)
        self.date = kw.get('date', '')
        self.url = kw.get('url')            # the zip on GitHub; None when only cached
        self.size = kw.get('size', 0)
        self.digest = kw.get('digest')      # 'sha256:...' as GitHub reports it
        self.installer_url = kw.get('installer_url')
        self.page = kw.get('page', RELEASES_PAGE)

    @property
    def zip_name(self):
        return f'armada2-refit-{self.version}.zip'

    @property
    def cached(self):
        return os.path.join(CACHE, 'releases', self.zip_name)

    def is_cached(self):
        return os.path.isfile(self.cached)

    def to_json(self):
        return dict(version=self.version, prerelease=self.prerelease, date=self.date,
                    url=self.url, size=self.size, digest=self.digest,
                    installer_url=self.installer_url, page=self.page)


def host_env():
    """The environment the installer was started in, as the user had it. The AppImage's
    AppRun points Python, GTK and the loader at its own copies and keeps each variable's
    old value in A2_ORIG_<NAME> (":unset" for none); anything we start -- install.sh,
    bash, the browser -- must not inherit those. Outside an AppImage this is os.environ."""
    env = dict(os.environ)
    if env.pop('A2_APPIMAGE', None):
        for k in [k for k in env if k.startswith('A2_ORIG_')]:
            v = env.pop(k)
            if v == ':unset':
                env.pop(k[8:], None)
            else:
                env[k[8:]] = v
    return env


CA_BUNDLES = ('/etc/ssl/certs/ca-certificates.crt', '/etc/pki/tls/certs/ca-bundle.crt',
              '/etc/ssl/ca-bundle.pem', '/etc/pki/tls/cacert.pem', '/etc/ssl/cert.pem')


def ssl_context():
    """The default context; a bundled Python's compiled-in certificate folder may not
    exist on this distribution, so fall back to the usual bundles."""
    ctx = ssl.create_default_context()
    if not ctx.cert_store_stats().get('x509_ca'):
        for f in CA_BUNDLES:
            if os.path.isfile(f):
                ctx.load_verify_locations(f)
                break
    return ctx


def http_get(url, timeout=15):
    req = urllib.request.Request(url, headers={
        'User-Agent': f'armada2-refit-installer/{INSTALLER_VERSION}',
        'Accept': 'application/vnd.github+json'})
    return urllib.request.urlopen(req, timeout=timeout, context=ssl_context())


def fetch_releases():
    """The releases on GitHub, newest first; also writes the list to the cache."""
    with http_get(API) as r:
        data = json.load(r)
    out = []
    for rel in data:
        if rel.get('draft'):
            continue
        version = rel.get('tag_name', '').lstrip('v')
        zips = [a for a in rel.get('assets', []) if a['name'] == f'armada2-refit-{version}.zip']
        if not zips:
            continue
        inst = [a for a in rel.get('assets', []) if a['name'] == INSTALLER_ASSET]
        out.append(Release(version, prerelease=bool(rel.get('prerelease')),
                           date=(rel.get('published_at') or '')[:10],
                           url=zips[0]['browser_download_url'], size=zips[0].get('size', 0),
                           digest=zips[0].get('digest'), page=rel.get('html_url', RELEASES_PAGE),
                           installer_url=inst[0]['browser_download_url'] if inst else None))
    save_json(os.path.join(CACHE, 'releases.json'),
              dict(checked=time.time(), releases=[r.to_json() for r in out]))
    return out


def cached_releases():
    """(releases, when last checked) from the cache, plus any zip in the cache that the
    list no longer names -- GitHub keeps only the newest three releases."""
    data = load_json(os.path.join(CACHE, 'releases.json'), {}) or {}
    rels = [Release(**r) for r in data.get('releases', [])]
    return merge_cached(rels), data.get('checked')


def merge_cached(rels):
    known = {r.version for r in rels}
    for z in glob.glob(os.path.join(CACHE, 'releases', 'armada2-refit-*.zip')):
        m = re.match(r'armada2-refit-([\d.]+)\.zip$', os.path.basename(z))
        if m and m.group(1) not in known:
            rels.append(Release(m.group(1), size=os.path.getsize(z)))
            known.add(m.group(1))
    return sorted(rels, key=lambda r: vtuple(r.version), reverse=True)


def default_release(rels):
    """The newest full release; a pre-release (not yet seen in game) only when there is
    nothing else."""
    full = [r for r in rels if not r.prerelease]
    return (full or rels or [None])[0]


def download(rel, progress=None):
    """The release zip into the cache, checked against GitHub's digest; returns its path."""
    if rel.is_cached():
        return rel.cached
    if not rel.url:
        raise RuntimeError(f'{rel.version} is neither cached nor on GitHub')
    os.makedirs(os.path.dirname(rel.cached), exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=os.path.dirname(rel.cached), suffix='.part')
    try:
        h = hashlib.sha256()
        with os.fdopen(fd, 'wb') as f, http_get(rel.url, timeout=30) as r:
            total = int(r.headers.get('Content-Length') or rel.size or 0)
            got = 0
            while True:
                chunk = r.read(64 << 10)
                if not chunk:
                    break
                f.write(chunk)
                h.update(chunk)
                got += len(chunk)
                if progress:
                    progress(got, total)
        if rel.digest and rel.digest.startswith('sha256:') and h.hexdigest() != rel.digest[7:]:
            raise RuntimeError(f'the download of {rel.version} is damaged (checksum)')
        Package(tmp)        # a zip we can read, with a package in it
        os.replace(tmp, rel.cached)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)
    return rel.cached


# ------------------------------------------------------------------ packages

class Package:
    """A release zip: its manifest (schema 1), or one built from README.txt (schema 0)."""
    def __init__(self, path):
        self.path = path
        with zipfile.ZipFile(path) as z:
            names = z.namelist()
            tops = {n.split('/', 1)[0] for n in names}
            if len(tops) != 1:
                raise RuntimeError('not a release package: more than one top folder')
            self.top = tops.pop()
            if not any(n == f'{self.top}/install.sh' for n in names):
                raise RuntimeError('not a release package: no install.sh')
            self.has = {n[len(self.top) + 1:] for n in names}
            try:
                self.manifest = json.loads(z.read(f'{self.top}/manifest.json'))
            except KeyError:
                self.manifest = self._legacy(z.read(f'{self.top}/README.txt').decode())
        m = self.manifest
        self.schema = int(m.get('schema', 0))
        self.version = m.get('version', '?')
        self.layers = m.get('layers', [])
        self.steps = m.get('steps', STEPS)

    def supported(self):
        return self.schema in SCHEMAS and self.for_system()

    def for_system(self):
        """Whether the package installs on this system. Linux: any. Windows and macOS:
        only a package whose manifest names the system (manifest `windows` / `macos`):
        before them install.ps1 printed no progress and install.sh did not run on a Mac."""
        return SYSTEM == 'linux' or isinstance(self.manifest.get(SYSTEM), dict)

    def why_not(self):
        if self.schema not in SCHEMAS:
            return (f'release {self.version} uses package schema {self.schema}; this installer '
                    f'reads {", ".join(map(str, SCHEMAS))} — download the newest installer')
        if not self.for_system():
            return f'release {self.version} cannot be installed on {SYSTEM_NAME} — pick a newer one'
        return None

    def command(self, pkgdir, args):
        """The argv that runs the package's installer here; args is [game] or
        ['--uninstall', game]."""
        m = self.manifest.get(SYSTEM) if isinstance(self.manifest.get(SYSTEM), dict) else {}
        if SYSTEM == 'windows':
            script = os.path.join(pkgdir, m.get('script', 'install.ps1'))
            args = [m.get('uninstall', '-Uninstall') if a == '--uninstall' else a for a in args]
            return ['powershell.exe', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
                    '-File', script] + args
        script = os.path.join(pkgdir, m.get('script', 'install.sh'))
        args = [m.get('uninstall', '--uninstall') if a == '--uninstall' else a for a in args]
        return ['/bin/bash' if SYSTEM == 'macos' else 'bash', script] + args

    def _legacy(self, readme):
        m = re.search(r'Armada II Refit (\S+)\s+built from commit (\S+)', readme)
        layers = [dict(id='prereqs', name='Widescreen patch', version='1.0', when='missing',
                       summary='STA2WidescreenPatch and the Ultimate ASI Loader that loads '
                               'every plugin')]
        cur = None
        for line in readme.splitlines():
            r = re.match(r'^  (\S+\.(?:asi|dll|conf))\s+(?:\+\s+\S+\s+)?[a-z]+ ([\d.]+)\s+(.*)$',
                         line)
            if r and r.group(1) in LEGACY_LAYERS:
                lid, name, when = LEGACY_LAYERS[r.group(1)]
                cur = dict(id=lid, name=name, version=r.group(2), when=when,
                           summary=r.group(3).strip())
                layers.append(cur)
            elif cur and re.match(r'^ {30,}\S', line):
                cur['summary'] += ' ' + line.strip()
            else:
                cur = None
        for l in layers:
            l['summary'] = re.sub(r'\s*--\s*', ' — ', l['summary']).rstrip(',')
        layers.append(dict(id='bloom', name='Bloom', version='', when='vkbasalt',
                           summary='MagicBloom through vkBasalt; its shaders are downloaded'))
        return dict(schema=0, version=m.group(1) if m else '?',
                    commit=m.group(2) if m else '', layers=layers, steps=STEPS)

    def unpack(self, dest):
        """Into dest/, refusing any member that would land outside it."""
        if os.path.isdir(dest):
            shutil.rmtree(dest)
        os.makedirs(dest)
        root = os.path.realpath(dest)
        with zipfile.ZipFile(self.path) as z:
            for n in z.namelist():
                t = os.path.realpath(os.path.join(dest, n))
                if t != root and not t.startswith(root + os.sep):
                    raise RuntimeError(f'unsafe path in the package: {n}')
            z.extractall(dest)
        return os.path.join(dest, self.top)


# ------------------------------------------------------------------ installing

class Cancelled(Exception):
    pass


class Job:
    """One install or uninstall, run on a worker thread. `report(fraction, step, detail)`
    and `log(line)` are called from that thread."""
    def __init__(self, report, log):
        self.report, self.log = report, log

    def _run_script(self, pkg, pkgdir, args, steps, lo, hi):
        env = dict(host_env(), A2_PROGRESS='1', A2_SKIP=' '.join(EXCLUDED))
        if FROZEN:
            # This program runs the scripts' Python steps itself (run_python).
            env['A2_PYTHON'] = sys.executable
        cmd = pkg.command(pkgdir, args)
        self.log('$ ' + ' '.join(cmd))
        proc = subprocess.Popen(cmd, cwd=pkgdir, env=env, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, text=True, bufsize=1,
                                encoding='utf-8', errors='replace', creationflags=NO_WINDOW)
        out, seen = [], 0
        for line in proc.stdout:
            line = line.rstrip('\n')
            step = None
            if line.startswith('::step '):
                step = line.split()[1] if len(line.split()) > 1 else None
            else:
                out.append(line)
                self.log(line)
                for mark, sid in LEGACY_MARKS:
                    if mark in line and sid in steps and steps.index(sid) >= seen:
                        step = sid
                        break
            if step:
                i = steps.index(step) + 1 if step in steps else seen + 1
                seen = max(seen, i)
                self.report(lo + (hi - lo) * min(seen, len(steps)) / len(steps), step, None)
        if proc.wait() != 0:
            tail = [l for l in out if l.strip()][-1:] or ['the install script failed']
            raise RuntimeError(tail[0])
        return out

    def install(self, rel, zip_path, game, launcher=True):
        """rel may be None when installing a local zip."""
        if game_running():
            raise RuntimeError('Armada II is running — quit the game first')
        if zip_path is None:
            ver = rel.version
            fresh = not rel.is_cached()
            self.report(0.0, 'download' if fresh else 'unpack', dict(version=ver))

            def dl(got, total):
                frac = got / total if total else 0
                self.report(0.30 * frac, 'download',
                            dict(version=ver, detail=f'{human(got)} / {human(total)}'))
            zip_path = download(rel, dl)
        pkg = Package(zip_path)
        if not pkg.supported():
            raise RuntimeError(pkg.why_not())
        self.report(0.32, 'unpack', None)
        pkgdir = pkg.unpack(os.path.join(CACHE, 'unpacked', pkg.version))
        out = self._run_script(pkg, pkgdir, [game], pkg.steps, 0.35, 0.92)
        if pkg.schema == 0 and 'cutscenes' in EXCLUDED:
            # Its install.sh knows no A2_SKIP: take the cutscene player out again.
            unproxy(game, self.log)

        # What install.sh asks of the launcher, as it printed it.
        wanted = {}
        for line in out:
            m = re.match(r'^ {4}([A-Z][A-Z0-9_]*)=(.*)$', line)
            if m:
                wanted[m.group(1)] = m.group(2)
        old = load_json(os.path.join(game, STATE_FILE), {}) or {}
        state = dict(version=pkg.version, schema=pkg.schema, installer=INSTALLER_VERSION,
                     installed=time.strftime('%Y-%m-%d %H:%M'), launcher_env=wanted,
                     heroic=old.get('heroic'), prefix=old.get('prefix'))
        notes = []
        g = next((x for x in find_games() if os.path.realpath(x.path) == os.path.realpath(game)),
                 None)
        if launcher and g and g.heroic and wanted:
            self.report(0.94, 'launcher', None)
            changes, conflicts = heroic_plan(g.heroic, wanted)
            notes += conflicts
            if changes is not None:
                if heroic_running():
                    notes.append('Heroic is running: it may write its own settings back — '
                                 'restart it to be sure it reads these')
                done = heroic_apply(g.heroic, changes)
                prev = (state['heroic'] or {}).get('changes', [])
                keys = {c['key'] for c in done}
                state['heroic'] = dict(config=g.heroic.config, app=g.heroic.app,
                                       changes=[c for c in prev if c['key'] not in keys] + done)
                for c in done:
                    self.log(f'Heroic: {c["key"]}={c["new"]}')
        elif launcher and g and g.prefix and wanted:
            self.report(0.94, 'launcher', None)
            changes, conflicts = prefix_plan(g.prefix, wanted)
            notes += conflicts
            if changes and wineserver_running():
                notes.append('Wine is running: the DLL overrides were not written into '
                             f'{g.prefix.label()} — quit every program in it and install again '
                             '(Launch game sets them anyway)')
            elif changes is not None:
                done = prefix_apply(g.prefix, changes)
                prev = (state['prefix'] or {}).get('changes', [])
                keys = {c['key'] for c in done}
                state['prefix'] = dict(userreg=g.prefix.userreg, label=g.prefix.label(),
                                       changes=[c for c in prev if c['key'] not in keys] + done)
                for c in done:
                    self.log(f'{g.prefix.label()}: DllOverrides {c["key"]}={c["new"]}')
        for n in notes:
            self.log('note: ' + n)
        save_json(os.path.join(game, STATE_FILE), state)
        self.report(1.0, 'done', None)
        return dict(version=pkg.version, env=wanted, notes=notes,
                    heroic=bool(state.get('heroic')), prefix=bool(state.get('prefix')))

    def uninstall(self, rels, game):
        if game_running():
            raise RuntimeError('Armada II is running — quit the game first')
        facts = inspect(game)
        state = facts['state'] or {}
        # The installed release's own script if we have it, else the newest we have.
        cands = sorted(rels, key=lambda r: (r.version != state.get('version'),
                                            not r.is_cached(), [-x for x in vtuple(r.version)]))
        if not cands:
            raise RuntimeError('no release package to uninstall with — refresh first')
        rel = cands[0]
        self.report(0.0, 'download' if not rel.is_cached() else 'unpack',
                    dict(version=rel.version))
        pkg = Package(download(rel))
        pkgdir = pkg.unpack(os.path.join(CACHE, 'unpacked', pkg.version))
        self._run_script(pkg, pkgdir, ['--uninstall', game], ['uninstall'], 0.1, 0.85)
        h = state.get('heroic')
        if h and os.path.exists(h.get('config', '')):
            self.report(0.9, 'restore', None)
            her = Heroic(os.path.dirname(os.path.dirname(h['config'])), h['app'], False)
            her.config = h['config']
            for c in heroic_apply(her, h.get('changes', []), undo=True):
                self.log(f'Heroic: {c["key"]} back to {c["old"] if c["old"] is not None else "(unset)"}')
        pr = state.get('prefix')
        if pr and os.path.isfile(pr.get('userreg', '')):
            if wineserver_running():
                self.log(f'note: Wine is running: the DLL overrides stay in {pr.get("label")}')
            else:
                self.report(0.92, 'restore', None)
                pre = WinePrefix(os.path.dirname(pr['userreg']), 'Wine', name=pr.get('label'))
                for c in prefix_apply(pre, pr.get('changes', []), undo=True):
                    self.log(f'{pr.get("label")}: DllOverrides {c["key"]} removed')
        try:
            os.unlink(os.path.join(game, STATE_FILE))
        except OSError:
            pass
        self.report(1.0, 'done', None)


# ------------------------------------------------------------------ settings

def settings():
    return load_json(SETTINGS, {}) or {}


def remember(**kw):
    s = settings()
    s.update(kw)
    save_json(SETTINGS, s)


# ------------------------------------------------------------------ command line

def cli(argv):
    import argparse
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--list', action='store_true')
    ap.add_argument('--install', nargs='?', const='', metavar='VERSION')
    ap.add_argument('--package', metavar='ZIP')
    ap.add_argument('--uninstall', action='store_true')
    ap.add_argument('--game')
    ap.add_argument('--no-launcher', action='store_true')
    ap.add_argument('--check-version')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--vkbasalt-howto', nargs='?', const='', metavar='FAMILY',
                    help='how to install vkBasalt here (or for FAMILY: arch, fedora, '
                         'fedora-ostree, debian, ubuntu, other, all)')
    ap.add_argument('--command', type=int, metavar='N',
                    help='with --vkbasalt-howto: print only its Nth command')
    a = ap.parse_args(argv)
    if a.check_version is not None:
        print(INSTALLER_VERSION)
        return 0 if a.check_version == INSTALLER_VERSION else 1
    if a.selftest:
        return selftest()
    if a.vkbasalt_howto is not None:
        fams = list(VKBASALT_HOWTO) if a.vkbasalt_howto == 'all' else [a.vkbasalt_howto or None]
        for fam in fams:
            if fam and fam not in VKBASALT_HOWTO:
                print(f'unknown family {fam}', file=sys.stderr)
                return 2
            name, steps = vkbasalt_howto(fam)
            if a.command:
                if not 1 <= a.command <= len(steps):
                    return 2
                print(steps[a.command - 1][1])
                continue
            print(f'{name}:' if len(fams) > 1 else f'Bloom needs a 32-bit vkBasalt. On {name}:')
            for text, cmd in steps:
                print(f'  {text}\n      {cmd}')
        return 0

    def releases():
        try:
            return merge_cached(fetch_releases())
        except Exception as e:
            print(f'offline ({e}); using the cache', file=sys.stderr)
            return cached_releases()[0]

    games = find_games()
    if a.list:
        print('games:')
        for g in games:
            facts = inspect(g.path)
            print(f'  {g.label()}' + (f'  [Heroic app {g.heroic.app}]' if g.heroic else '')
                  + f'  installed: {facts["installed"] or "no"}')
        print('releases:')
        for r in releases():
            print(f'  {r.version:10} {r.date:10} {"pre-release" if r.prerelease else "release":11}'
                  f' {"cached" if r.is_cached() else ""}')
        return 0
    game = a.game or (games[0].path if games else None)
    if not game or not exe_in(game):
        print('no Armada II found -- pass --game DIR', file=sys.stderr)
        return 1
    last = [None]

    def report(frac, step, info):
        text = STATUS.get(step, step).format(**(info or {'version': ''}))
        if (step, text) != last[0]:
            print(f'[{frac * 100:3.0f}%] {text}…', file=sys.stderr)
            last[0] = (step, text)

    job = Job(report, print)
    try:
        if a.uninstall:
            job.uninstall(releases(), game)
        elif a.package:
            job.install(None, a.package, game, launcher=not a.no_launcher)
        elif a.install is not None:
            rels = releases()
            rel = next((r for r in rels if r.version == a.install), None) if a.install \
                else default_release(rels)
            if not rel:
                print(f'no release {a.install}', file=sys.stderr)
                return 1
            job.install(rel, None, game, launcher=not a.no_launcher)
        else:
            ap.print_help()
    except Exception as e:
        print(f'error: {e}', file=sys.stderr)
        return 1
    return 0


def selftest():
    """The parts that need neither a display nor the network."""
    ok = True

    def check(cond, what):
        nonlocal ok
        print(('ok    ' if cond else 'FAIL  ') + what)
        ok = ok and cond

    with tempfile.TemporaryDirectory() as t:
        cfg = os.path.join(t, 'GamesConfig')
        os.makedirs(cfg)
        h = Heroic(t, GOG_APP, False)
        save_json(h.config, {GOG_APP: {'enviromentOptions': [
            {'key': 'WINEDLLOVERRIDES', 'value': 'winmm=n,b'},
            {'key': 'ENABLE_VKBASALT', 'value': '0'}]}, 'version': 'v0'})
        want = {'WINEDLLOVERRIDES': 'winmm=n,b;d3d8=n,b', 'ENABLE_VKBASALT': '1',
                'VKBASALT_CONFIG_FILE': '/x/vkBasalt.conf'}
        changes, conflicts = heroic_plan(h, want)
        check([c['key'] for c in changes] == ['WINEDLLOVERRIDES', 'VKBASALT_CONFIG_FILE'],
              'plan: adds the missing override and the unset variable')
        check(len(conflicts) == 1 and 'ENABLE_VKBASALT' in conflicts[0],
              'plan: a variable set otherwise is a conflict, not overwritten')
        heroic_apply(h, changes)
        env = {e['key']: e['value'] for e in load_json(h.config)[GOG_APP]['enviromentOptions']}
        check(env['WINEDLLOVERRIDES'] == 'winmm=n,b;d3d8=n,b', 'apply: overrides merged')
        check(os.path.exists(h.config + '.a2refit-backup'), 'apply: backup kept')
        check(heroic_plan(h, want)[0] == [], 'plan: nothing left to do after apply')
        heroic_apply(h, changes, undo=True)
        env = {e['key']: e['value'] for e in load_json(h.config)[GOG_APP]['enviromentOptions']}
        check(env == {'WINEDLLOVERRIDES': 'winmm=n,b', 'ENABLE_VKBASALT': '0'},
              'undo: back to what it was')
        check(heroic_plan(Heroic(t, '123', False), want)[0] is None,
              'plan: no settings file is reported, not created')

        readme = ('Armada II Refit 11.4.0            built from commit abc1234\n\n'
                  'WHAT GOES IN (game/, copied beside Armada2.exe)\n'
                  '  HUD.asi      + HUD.ini        hud 2.3.0        in-game HUD, font\n'
                  '  MSAA.asi     + MSAA.ini       msaa 1.0.0       multisample anti-aliasing -- installed only\n'
                  '  dxvk.conf                     postfx 1.0.0     16x anisotropic filtering, LOD bias, seamless\n')
        z = os.path.join(t, 'armada2-refit-11.4.0.zip')
        with zipfile.ZipFile(z, 'w') as zf:
            zf.writestr('armada2-refit-11.4.0/README.txt', readme)
            zf.writestr('armada2-refit-11.4.0/install.sh', '#!/bin/sh\n')
        p = Package(z)
        check(p.schema == 0 and p.version == '11.4.0', 'legacy zip: schema 0, version from README')
        check([l['id'] for l in p.layers] == ['prereqs', 'hud', 'msaa', 'renderer', 'bloom'],
              'legacy zip: layers from README')
        nw, nh = NEBULA_SIZE
        neb = nebula_pixels(7)
        alpha = neb[3::4]
        check(len(neb) == nw * nh * 4 and 0.2 < sum(1 for v in alpha if v) / len(alpha) < 0.8,
              'nebula: the size asked for, and part of the sky clouded')

        def seam(a, b):         # mean alpha step between columns a and b
            return sum(abs(alpha[r * nw + a] - alpha[r * nw + b]) for r in range(nh)) / nh
        check(seam(nw - 1, 0) <= 3 * seam(nw // 2, nw // 2 + 1) + 2,
              'nebula: tiles across without a seam')
        check(heroic_launch_uri(Heroic(t, GOG_APP, False)) ==
              f'heroic://launch?appName={GOG_APP}&runner=gog', "launch: Heroic's own URI")
        check(split_overrides('a=n,b; b=n') == {'a': 'n,b', 'b': 'n'}, 'overrides parse')

        # A Wine prefix's registry (macOS): written, kept, and taken back out byte for byte.
        reg = os.path.join(t, 'user.reg')
        stock = ('WINE REGISTRY Version 2\n;; All keys relative to \\User\n\n#arch=win64\n\n'
                 '[Software\\\\Wine\\\\AppDefaults\\\\Armada2.exe\\\\X11 Driver] 1700000000\n'
                 '"Decorated"="N"\n\n[Software\\\\Wine\\\\DllOverrides] 1700000000\n'
                 '"*d3d8"="builtin"\n')
        with open(reg, 'w', newline='') as f:
            f.write(stock)
        pre = WinePrefix(t, 'CrossOver', name='Armada')
        want = {'WINEDLLOVERRIDES': 'winmm=n,b;d3d8=n,b'}
        ch, conf = prefix_plan(pre, want)
        check([c['key'] for c in ch] == ['winmm', 'd3d8'] and not conf,
              'prefix: both overrides planned for the game (the bottle-wide one is not the game\'s)')
        prefix_apply(pre, ch)
        lines = read_reg(reg)
        sec = reg_section(lines)
        check(sec and reg_values(lines, sec) == {'winmm': 'native,builtin', 'd3d8': 'native,builtin'},
              'prefix: AppDefaults\\Armada2.exe\\DllOverrides written')
        check(prefix_plan(pre, want)[0] == [], 'prefix: nothing left to do after apply')
        check(os.path.exists(reg + '.a2refit-backup'), 'prefix: backup kept')
        prefix_apply(pre, ch, undo=True)
        check(open(reg, newline='').read() == stock, 'prefix: undo leaves user.reg as it was')
        check(prefix_plan(WinePrefix(os.path.join(t, 'none'), 'Wine'), want)[0] is None,
              'prefix: no registry is reported, not created')
        check(reg_mode('n,b') == 'native,builtin' and reg_mode('n') == 'native', 'prefix: modes')
        check(prefix_of(os.path.join(t, 'drive_c', 'GOG Games', 'Armada')) is not None and
              os.path.realpath(prefix_of(os.path.join(t, 'drive_c', 'x')).root) == os.path.realpath(t),
              'prefix: found above drive_c')
        check(vtuple('11.10.0') > vtuple('11.9.3'), 'versions sort as numbers')
        keep = {k: os.environ.get(k) for k in ('A2_APPIMAGE', 'PATH', 'PYTHONHOME', 'A2_ORIG_PATH',
                                              'A2_ORIG_PYTHONHOME')}
        os.environ.update(A2_APPIMAGE='1', PATH='/bundle/bin:/usr/bin', PYTHONHOME='/bundle',
                          A2_ORIG_PATH='/usr/bin', A2_ORIG_PYTHONHOME=':unset')
        he = host_env()
        check(he['PATH'] == '/usr/bin' and 'PYTHONHOME' not in he and 'A2_APPIMAGE' not in he
              and not any(k.startswith('A2_ORIG_') for k in he),
              "host_env: the AppImage's variables are put back, and the markers removed")
        for k, v in keep.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        if keep['A2_APPIMAGE'] is None:
            check(host_env().get('PATH') == os.environ.get('PATH'),
                  'host_env: unchanged outside an AppImage')
        for osr, fam in [({'ID': 'arch'}, 'arch'), ({'ID': 'cachyos', 'ID_LIKE': 'arch'}, 'arch'),
                         ({'ID': 'fedora'}, 'fedora'),
                         ({'ID': 'fedora', 'VARIANT_ID': 'silverblue'}, 'fedora-ostree'),
                         ({'ID': 'linuxmint', 'ID_LIKE': 'ubuntu debian'}, 'ubuntu'),
                         ({'ID': 'pop', 'ID_LIKE': 'ubuntu debian'}, 'ubuntu'),
                         ({'ID': 'debian'}, 'debian'), ({'ID': 'nixos'}, 'other')]:
            if fam == 'fedora' and os.path.exists('/run/ostree-booted'):
                continue
            check(distro_family(osr) == fam, f'distro: {osr} is {fam}')
    return 0 if ok else 1



# ------------------------------------------------------------------ the window

# Amber, the warm of the nebula's cores, in place of the desktop's accent: libadwaita's
# named colours for everything that takes the accent, and the two that matter set
# outright, so they hold on any libadwaita.
AMBER, AMBER_LIGHT, AMBER_INK = '#e8922f', '#f5a849', '#1c1206'
CSS = """
@define-color accent_bg_color AMBER;
@define-color accent_color AMBER_LIGHT;
@define-color accent_fg_color AMBER_INK;
button.suggested-action { background-color: AMBER; color: AMBER_INK; }
progressbar progress { background-color: AMBER_LIGHT; }
window.a2 { background: #05070d; }
.sky-card { background: alpha(#10141f, 0.74); border: 1px solid alpha(white, 0.08);
            border-radius: 18px; }
.sky-card list, .sky-card row { background: transparent; }
.sky-card row { border-radius: 0; }
.sky-card > list > row:first-child { border-radius: 18px 18px 0 0; }
.sky-card > list > row:last-child { border-radius: 0 0 18px 18px; }
.title-main { font-size: 40px; font-weight: 900; letter-spacing: 3px;
              text-shadow: 0 0 18px alpha(#8fb4ff, 0.35); }
.title-tag { color: AMBER_LIGHT; font-size: 14px; font-weight: 800; letter-spacing: 0.9em;
             margin-left: 0.9em; text-shadow: 0 0 10px alpha(AMBER, 0.55); }
.status { color: alpha(@window_fg_color, 0.75); }
.status.err { color: @error_color; }
.status.ok { color: @success_color; }
.heads-up { color: alpha(@window_fg_color, 0.7); font-size: 0.92em; }
.heads-up.warn { color: @warning_color; }
progressbar.thin trough, progressbar.thin progress { min-height: 4px; }
progressbar.thin trough { background: alpha(white, 0.10); }
.mono { font-family: monospace; font-size: 0.9em; }
.logview { background: alpha(black, 0.35); padding: 8px; }
""".replace('AMBER_LIGHT', AMBER_LIGHT).replace('AMBER_INK', AMBER_INK).replace('AMBER', AMBER)
# libadwaita 1.6 reads the accent from CSS variables, which GTK parses from 4.16 on.
CSS_VARS = (f':root {{ --accent-bg-color: {AMBER}; --accent-color: {AMBER_LIGHT}; '
            f'--accent-fg-color: {AMBER_INK}; }}\n')


NEBULA_SIZE = (256, 160)


def nebula_pixels(seed, w=NEBULA_SIZE[0], h=NEBULA_SIZE[1]):
    """A nebula for the window's sky, as premultiplied ARGB32 (cairo's byte order): value
    noise, gently domain-warped into soft billows (no thin filaments: they read as
    electric), a second field for dark dust lanes and a slow one that moves the hue between
    violet and teal; warm where it is densest. The lattices are periodic, so it tiles and can drift without a seam.
    Pure Python (no numpy in the AppImage): about half a second, so run it off the
    main thread."""
    import math
    import random
    rnd = random.Random(seed)
    octs = []
    for o in range(5):
        cx = 4 << o
        cy = max(2, cx * h // w)
        octs.append((cx, cy, [[rnd.random() for _ in range(cx)] for _ in range(cy)]))

    def noise(lat, x, y):
        cx, cy, g = lat
        x, y = x * cx, y * cy
        xi, yi = math.floor(x), math.floor(y)
        fx, fy = x - xi, y - yi
        fx, fy = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy)
        x0, x1 = xi % cx, (xi + 1) % cx
        r0, r1 = g[yi % cy], g[(yi + 1) % cy]
        a = r0[x0] + (r0[x1] - r0[x0]) * fx
        b = r1[x0] + (r1[x1] - r1[x0]) * fx
        return a + (b - a) * fy

    def fbm(x, y, n=5, ofs=0.0):
        v, amp, tot = 0.0, 1.0, 0.0
        for o in range(n):
            v += amp * noise(octs[o], x + ofs, y + ofs * 0.7)
            tot += amp
            amp *= 0.5
        return v / tot

    cells = []
    for j in range(h):
        y = j / h
        for i in range(w):
            x = i / w
            qx, qy = fbm(x, y, 4, 0.31), fbm(x, y, 4, 0.67)
            cells.append((fbm(x + 0.35 * qx, y + 0.35 * qy), qx,
                          noise(octs[0], x + 0.5, y + 0.2)))
    dens = sorted(c[0] for c in cells)
    # The thinnest 40 % stays empty; the top is a percentile, not the brightest cell, so
    # every launch comes out about as bright, and none of it clips behind the cards.
    lo, hi = dens[len(dens) * 2 // 5], dens[len(dens) * 995 // 1000]
    violet, teal = (0.30, 0.18, 0.75), (0.10, 0.45, 0.80)
    buf = bytearray(w * h * 4)
    for k, (d, dust, hue) in enumerate(cells):
        t = min(1.0, max(0.0, (d - lo) / (hi - lo))) ** 1.6
        t *= 1 - 0.7 * min(1.0, max(0.0, (dust - 0.5) * 5))
        m = min(1.0, max(0.0, (hue - 0.3) * 2.5))
        hot = max(0.0, t - 0.6) * 2.5
        r = min(1.0, violet[0] + (teal[0] - violet[0]) * m + 0.45 * hot)
        g = min(1.0, violet[1] + (teal[1] - violet[1]) * m + 0.30 * hot)
        b = min(1.0, violet[2] + (teal[2] - violet[2]) * m + 0.05 * hot)
        a = min(0.6, t * 0.9)
        buf[4 * k:4 * k + 4] = bytes((int(255 * b * a), int(255 * g * a),
                                      int(255 * r * a), int(255 * a)))
    return buf


def run_gui():
    import gi
    gi.require_version('Gtk', '4.0')
    gi.require_version('Adw', '1')
    from gi.repository import Adw, Gdk, Gio, GLib, Gtk
    import math
    import random

    def lab(text='', cls=(), xalign=0.0, wrap=False, **kw):
        w = Gtk.Label(label=text, xalign=xalign, wrap=wrap, **kw)
        for c in ([cls] if isinstance(cls, str) else cls):
            w.add_css_class(c)
        return w

    class Sky(Gtk.DrawingArea):
        """The window's background: a dark gradient, a nebula made up anew at every
        launch (nebula_pixels, faded in once it is ready) and three layers of stars, all
        drifting at different speeds. While an install runs it goes to warp (set_warp):
        everything speeds up and the stars draw as streaks, easing in and out. Drawn in
        code, so there is nothing to ship; still when the desktop has animations off."""
        LAYERS = ((90, 0.55, 3.0), (60, 0.85, 7.0), (28, 1.35, 14.0))   # count, radius, px/s
        CLOUD_SPEED = 1.5       # px/s on screen: behind the slowest stars
        CLOUD_FADE = 1.5        # s
        WARP_SPEED = 40.0       # times the drift at full warp
        WARP_EASE = 1.2         # s, about, to reach warp or to drop out of it
        WARP_HOLD = 3.0         # s at least: an install from the cache takes under one

        def __init__(self):
            super().__init__(hexpand=True, vexpand=True)
            rnd = random.Random(2399)
            self.stars = [[rnd.random(), rnd.random(), r, rnd.uniform(0.35, 1.0),
                           rnd.uniform(0, 6.3), speed, rnd.choice((0, 0, 0, 1, 2))]
                          for count, r, speed in self.LAYERS for _ in range(count)]
            self.tints = ((1.0, 1.0, 1.0), (0.75, 0.85, 1.0), (1.0, 0.88, 0.75))
            self.t0 = None
            self.nebula = None
            self.cloud, self.cloud_t, self.cloud_scaled = None, None, None
            seed = random.SystemRandom().randrange(1 << 30)
            threading.Thread(target=lambda: GLib.idle_add(self.cloud_ready, nebula_pixels(seed)),
                             daemon=True).start()
            self.animate = Gtk.Settings.get_default().get_property('gtk-enable-animations')
            # Distance travelled, in seconds of ordinary drift: positions follow it rather
            # than the clock, so changing speed never makes anything jump.
            self.travel, self.warp, self.warp_to, self.t_last = 0.0, 0.0, 0.0, None
            self.warp_from, self.warp_off = 0.0, None
            self.set_draw_func(self.draw)
            if self.animate:
                self.add_tick_callback(self.tick)
            self._last = 0

        def tick(self, widget, clock):
            now = clock.get_frame_time()
            # About 30 frames a second is plenty for a drift; at warp, every frame.
            if now - self._last >= 33000 or self.warp > 0.001 or self.warp_to:
                self._last = now
                self.queue_draw()
            return GLib.SOURCE_CONTINUE

        def set_warp(self, on):
            t = GLib.get_monotonic_time() / 1e6
            if on and self.animate:
                self.warp_to, self.warp_from, self.warp_off = 1.0, t, None
            elif self.warp_to:
                self.warp_off = max(t, self.warp_from + self.WARP_HOLD)

        def advance(self):
            """Move the clock on: ease the warp level, add to the distance travelled."""
            t = GLib.get_monotonic_time() / 1e6
            dt = 0.0 if self.t_last is None else min(0.1, t - self.t_last)
            self.t_last = t
            if self.warp_off is not None and t >= self.warp_off:
                self.warp_to, self.warp_off = 0.0, None
            self.warp += (self.warp_to - self.warp) * min(1.0, dt * 3.0 / self.WARP_EASE)
            if abs(self.warp - self.warp_to) < 0.001:
                self.warp = self.warp_to
            self.travel += dt * self.speed()
            return t, dt

        def speed(self):
            e = self.warp * self.warp * (3 - 2 * self.warp)          # smoothstep
            return 1.0 + (self.WARP_SPEED - 1.0) * e

        def cloud_ready(self, buf):
            import cairo
            w, h = NEBULA_SIZE
            self.cloud = cairo.ImageSurface.create_for_data(buf, cairo.FORMAT_ARGB32, w, h,
                                                            w * 4)
            self.cloud_t = GLib.get_monotonic_time() / 1e6
            self.queue_draw()
            return GLib.SOURCE_REMOVE

        def build_nebula(self, w, h):
            import cairo
            s = cairo.ImageSurface(cairo.FORMAT_ARGB32, w, h)
            cr = cairo.Context(s)
            g = cairo.LinearGradient(0, 0, 0, h)
            g.add_color_stop_rgb(0, 0.016, 0.022, 0.050)
            g.add_color_stop_rgb(1, 0.030, 0.040, 0.085)
            cr.set_source(g)
            cr.paint()
            self.nebula = (w, h, s)

        def draw(self, area, cr, w, h):
            import cairo
            if not self.nebula or self.nebula[:2] != (w, h):
                self.build_nebula(w, h)
            cr.set_source_surface(self.nebula[2], 0, 0)
            cr.paint()
            now, dt = self.advance() if self.animate else (0.0, 0.0)
            travel, mult = self.travel, self.speed()
            if self.cloud:
                # Scaled to the window's height once per height, then only moved, by whole
                # pixels: filtering it every frame cost half again the sky's CPU time.
                if not self.cloud_scaled or self.cloud_scaled[0] != h:
                    sc = h / NEBULA_SIZE[1]
                    tw = max(1, round(NEBULA_SIZE[0] * sc))
                    tile = cairo.ImageSurface(cairo.FORMAT_ARGB32, tw, h)
                    tc = cairo.Context(tile)
                    tc.scale(tw / NEBULA_SIZE[0], sc)
                    src = cairo.SurfacePattern(self.cloud)
                    src.set_extend(cairo.EXTEND_REPEAT)     # so the edges wrap, too
                    src.set_filter(cairo.FILTER_GOOD)
                    tc.set_source(src)
                    tc.paint()
                    self.cloud_scaled = (h, tile)
                pat = cairo.SurfacePattern(self.cloud_scaled[1])
                pat.set_extend(cairo.EXTEND_REPEAT)
                pat.set_filter(cairo.FILTER_FAST)
                pat.set_matrix(cairo.Matrix(x0=-round(travel * self.CLOUD_SPEED)))
                cr.set_source(pat)
                fade = min(1.0, (now - self.cloud_t) / self.CLOUD_FADE) if self.animate else 1.0
                cr.paint_with_alpha(fade)
            # At warp a star is a streak behind it: at least what it covers in a frame, so
            # it reads as one motion and not a dotted line.
            streak = 0.0 if mult < 1.05 else max(dt * 1.6, 0.12)
            cr.set_line_cap(cairo.LINE_CAP_ROUND)
            for x, y, r, a, ph, speed, tint in self.stars:
                px = (x * w + travel * speed) % w
                tw = 0.75 + 0.25 * math.sin(now * 0.9 + ph) if self.animate else 1.0
                length = speed * mult * streak
                if length > 1.0:
                    # Faint at the tail, the star's own brightness at the head.
                    g = cairo.LinearGradient(px - length, 0, px, 0)
                    g.add_color_stop_rgba(0, *self.tints[tint], 0)
                    g.add_color_stop_rgba(1, *self.tints[tint], a * tw)
                    cr.set_source(g)
                    cr.set_line_width(2 * r)
                    cr.move_to(px - length, y * h)
                    cr.line_to(px, y * h)
                    cr.stroke()
                else:
                    cr.set_source_rgba(*self.tints[tint], a * tw)
                    cr.arc(px, y * h, r, 0, 6.2832)
                    cr.fill()

    class EngineBar(Gtk.Overlay):
        """The progress bar with an engine at its tip: an amber glow that flickers and a
        short trail behind it. The bar glides to each new fraction instead of jumping,
        and the glow fades out once it is full. A plain bar when animations are off."""
        HEIGHT = 24             # room for the glow around a 4 px bar
        GLIDE = 4.0             # 1/s: how fast the bar closes on a new fraction

        def __init__(self):
            super().__init__()
            self.bar = Gtk.ProgressBar(valign=Gtk.Align.CENTER, hexpand=True)
            self.bar.add_css_class('thin')
            box = Gtk.Box(height_request=self.HEIGHT)
            box.append(self.bar)
            self.set_child(box)
            self.glow = Gtk.DrawingArea(can_target=False)
            self.glow.set_draw_func(self.draw)
            self.add_overlay(self.glow)
            self.shown = self.target = self.lit = 0.0
            self.t_last = None
            self.animate = Gtk.Settings.get_default().get_property('gtk-enable-animations')
            if self.animate:
                self.add_tick_callback(self.tick)

        def set_fraction(self, f):
            if f < self.shown or not self.animate:
                self.shown = f          # a new job starts from nothing at once
                self.bar.set_fraction(f)
            self.target = f
            self.glow.queue_draw()

        def tick(self, widget, clock):
            t = clock.get_frame_time() / 1e6
            dt = 0.0 if self.t_last is None else min(0.1, t - self.t_last)
            self.t_last = t
            lit_to = 1.0 if 0.0 < self.target < 1.0 else 0.0
            if abs(self.target - self.shown) > 1e-4 or self.lit != lit_to or self.lit > 0:
                self.shown += (self.target - self.shown) * min(1.0, dt * self.GLIDE)
                self.lit += (lit_to - self.lit) * min(1.0, dt * (5.0 if lit_to else 1.5))
                if self.lit < 0.01 and not lit_to:
                    self.lit = 0.0
                self.bar.set_fraction(self.shown)
                self.glow.queue_draw()
            return GLib.SOURCE_CONTINUE

        def draw(self, area, cr, w, h):
            import cairo
            if self.lit <= 0.0 or self.shown <= 0.0:
                return
            x, y = self.shown * w, h / 2
            t = GLib.get_monotonic_time() / 1e6
            flick = 0.85 + 0.1 * math.sin(t * 23) + 0.05 * math.sin(t * 61)
            # The trail: the bar burning brighter toward the engine.
            trail = min(x, 80.0)
            g = cairo.LinearGradient(x - trail, 0, x, 0)
            g.add_color_stop_rgba(0, 1.0, 0.75, 0.40, 0)
            g.add_color_stop_rgba(1, 1.0, 0.85, 0.60, 0.8 * self.lit)
            cr.set_source(g)
            cr.rectangle(x - trail, y - 2, trail, 4)
            cr.fill()
            # The engine: white-hot at the core, amber around it.
            r = 14 * flick
            g = cairo.RadialGradient(x, y, 0, x, y, r)
            g.add_color_stop_rgba(0, 1.0, 0.97, 0.88, self.lit)
            g.add_color_stop_rgba(0.25, 1.0, 0.72, 0.32, 0.75 * self.lit * flick)
            g.add_color_stop_rgba(1, 0.96, 0.57, 0.18, 0)
            cr.set_source(g)
            cr.arc(x, y, r, 0, 6.2832)
            cr.fill()

    class Window(Adw.ApplicationWindow):
        def __init__(self, app):
            super().__init__(application=app, title='Armada II Refit',
                             default_width=480, default_height=700)
            self.add_css_class('a2')
            self.games, self.releases, self.package, self.facts = [], [], None, None
            self.busy, self.updating, self.user_picked = False, False, False
            self.checked, self.check_error = None, None
            self.banner_uri = RELEASES_PAGE
            self.build()
            self.load_games()
            self.refresh()

        # ---------------------------------------------------------- layout
        def build(self):
            overlay = Gtk.Overlay()
            self.sky = Sky()
            overlay.set_child(self.sky)

            view = Adw.ToolbarView(extend_content_to_top_edge=True)
            view.set_top_bar_style(Adw.ToolbarStyle.FLAT)
            header = Adw.HeaderBar(show_title=False)
            header.add_css_class('flat')
            menu = Gio.Menu()
            menu.append('Refresh releases', 'win.refresh')
            menu.append('Installation log', 'win.log')
            menu.append('Releases on GitHub', 'win.releases')
            header.pack_end(Gtk.MenuButton(icon_name='open-menu-symbolic', menu_model=menu))
            view.add_top_bar(header)
            self.banner = Adw.Banner(button_label='Download', revealed=False)
            self.banner.connect('button-clicked', lambda _b: self.open_uri(self.banner_uri))
            view.add_top_bar(self.banner)

            col = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18, valign=Gtk.Align.CENTER,
                          margin_top=8, margin_bottom=28, margin_start=16, margin_end=16)
            title = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
            title.append(lab('ARMADA II', 'title-main', xalign=0.5))
            # The tracking also follows the last letter: the margin puts it back in the middle.
            title.append(lab('REFIT', 'title-tag', xalign=0.5))
            col.append(title)

            card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
            card.add_css_class('sky-card')
            rows = Gtk.ListBox(selection_mode=Gtk.SelectionMode.NONE)
            card.append(rows)

            self.row_game = Adw.ActionRow(title='Game', subtitle_lines=2)
            self.dd_game = Gtk.DropDown(valign=Gtk.Align.CENTER)
            self.dd_game.connect('notify::selected', lambda *_: self.on_game())
            browse = Gtk.Button(icon_name='folder-open-symbolic', valign=Gtk.Align.CENTER,
                                tooltip_text='Choose the folder holding Armada2.exe')
            browse.add_css_class('flat')
            browse.connect('clicked', lambda _b: self.on_browse())
            self.row_game.add_suffix(self.dd_game)
            self.row_game.add_suffix(browse)
            rows.append(self.row_game)

            self.row_rel = Adw.ActionRow(title='Version', subtitle_lines=2)
            self.dd_rel = Gtk.DropDown(valign=Gtk.Align.CENTER)
            self.dd_rel.connect('notify::selected', lambda *_: self.on_release())
            self.row_rel.add_suffix(self.dd_rel)
            rows.append(self.row_rel)

            # Always written: the game does not start right without them.
            self.row_heroic = Adw.ActionRow(title='Heroic launch settings', subtitle_lines=3)
            rows.append(self.row_heroic)

            self.row_get_heroic = Adw.ExpanderRow(
                title='Install Heroic Games Launcher',
                subtitle='The refit runs the GOG game through Heroic, which also gets its '
                         'launch settings from here.', subtitle_lines=0)
            rows.append(self.row_get_heroic)
            self.get_heroic_rows = []

            self.row_env = Adw.ExpanderRow(title='Launch variables',
                                           subtitle='Set these in your launcher')
            self.env_text = Gtk.Label(xalign=0, selectable=True, wrap=True, hexpand=True)
            self.env_text.add_css_class('mono')
            erow = Adw.ActionRow()
            box = Gtk.Box(spacing=8, margin_top=8, margin_bottom=8, hexpand=True)
            box.append(self.env_text)
            cp = Gtk.Button(icon_name='edit-copy-symbolic', valign=Gtk.Align.START,
                            tooltip_text='Copy')
            cp.add_css_class('flat')
            cp.connect('clicked', lambda _b: self.copy(self.env_text.get_label()))
            box.append(cp)
            erow.set_child(box)
            self.row_env.add_row(erow)
            rows.append(self.row_env)

            self.row_bloom = Adw.ExpanderRow(title='Bloom needs vkBasalt',
                                             subtitle='Optional. Everything else installs without it.')
            rows.append(self.row_bloom)
            self.bloom_rows = []
            col.append(card)

            self.heads_up = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
            col.append(self.heads_up)

            act = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
            pair = Gtk.Box(spacing=12, halign=Gtk.Align.CENTER, homogeneous=True)
            self.b_install = Gtk.Button(label='Install')
            self.b_install.add_css_class('pill')
            self.b_install.set_size_request(190, 48)
            self.b_install.connect('clicked', lambda _b: self.on_install())
            pair.append(self.b_install)
            self.b_launch = Gtk.Button(child=Adw.ButtonContent(
                icon_name='media-playback-start-symbolic', label='Launch game'),
                tooltip_text='Start Armada II through Heroic')
            self.b_launch.add_css_class('pill')
            self.b_launch.set_size_request(190, 48)
            self.b_launch.connect('clicked', lambda _b: self.on_launch())
            pair.append(self.b_launch)
            act.append(pair)
            self.b_uninstall = Gtk.Button(label='Uninstall', halign=Gtk.Align.CENTER)
            self.b_uninstall.add_css_class('flat')
            self.b_uninstall.connect('clicked', lambda _b: self.on_uninstall())
            act.append(self.b_uninstall)
            col.append(act)

            prog = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
            self.bar = EngineBar()
            prog.append(self.bar)
            self.status = lab('', 'status', xalign=0.5, wrap=True, justify=Gtk.Justification.CENTER)
            prog.append(self.status)
            col.append(prog)

            clamp = Adw.Clamp(maximum_size=480, tightening_threshold=400, child=col)
            scroll = Gtk.ScrolledWindow(hscrollbar_policy=Gtk.PolicyType.NEVER, child=clamp)
            view.set_content(scroll)
            overlay.add_overlay(view)
            self.set_content(overlay)

            self.logview = Gtk.TextView(editable=False, monospace=True, cursor_visible=False,
                                        wrap_mode=Gtk.WrapMode.WORD_CHAR)
            self.logview.add_css_class('logview')
            self.logscroll = Gtk.ScrolledWindow(child=self.logview, vexpand=True)
            self.logdialog = None

            for name, fn, accel in (('refresh', self.refresh, 'F5'),
                                    ('log', self.show_log, '<Control>l'),
                                    ('install', self.on_install, '<Control>Return'),
                                    ('releases', lambda: self.open_uri(RELEASES_PAGE), None)):
                a = Gio.SimpleAction.new(name, None)
                a.connect('activate', lambda _a, _p, f=fn: f())
                self.add_action(a)
                if accel:
                    self.get_application().set_accels_for_action('win.' + name, [accel])

        # ---------------------------------------------------------- small things
        def open_uri(self, uri):
            Gtk.UriLauncher.new(uri).launch(self, None, None, None)

        def copy(self, text):
            self.get_clipboard().set(text)
            self.set_status('Copied to the clipboard', 'ok')

        def log(self, line):
            buf = self.logview.get_buffer()
            buf.insert(buf.get_end_iter(), line + '\n')
            adj = self.logscroll.get_vadjustment()
            GLib.idle_add(lambda: adj.set_value(adj.get_upper()))

        def show_log(self):
            if self.logdialog is None:
                self.logdialog = Adw.Dialog(title='Installation log', content_width=640,
                                            content_height=420)
                tv = Adw.ToolbarView()
                tv.add_top_bar(Adw.HeaderBar())
                tv.set_content(self.logscroll)
                self.logdialog.set_child(tv)
                self.logdialog.connect('closed', lambda _d: setattr(self, 'logdialog', None) or
                                       tv.set_content(None))
            self.logdialog.present(self)

        def set_status(self, text, kind=''):
            self.status.set_label(text)
            for c in ('err', 'ok'):
                self.status.remove_css_class(c)
            if kind:
                self.status.add_css_class(kind)

        def set_busy(self, busy):
            self.busy = busy
            for w in (self.dd_game, self.dd_rel):
                w.set_sensitive(not busy)
            self.update_buttons()

        def game(self):
            i = self.dd_game.get_selected()
            return self.games[i] if 0 <= i < len(self.games) else None

        def release(self):
            i = self.dd_rel.get_selected()
            return self.releases[i] if 0 <= i < len(self.releases) else None

        def in_thread(self, fn, done=None, fail=None):
            def run():
                try:
                    r = fn()
                except Exception as e:
                    GLib.idle_add(fail or self.failed, e)
                else:
                    if done:
                        GLib.idle_add(done, r)
            threading.Thread(target=run, daemon=True).start()

        def failed(self, e):
            self.set_busy(False)
            self.sky.set_warp(False)
            self.set_status(f'Failed: {e}', 'err')
            self.log(f'error: {e}')

        # ---------------------------------------------------------- the game
        def load_games(self, select=None):
            self.games = find_games()
            last = select or settings().get('game')
            if last and exe_in(last) and not any(
                    os.path.realpath(g.path) == os.path.realpath(last) for g in self.games):
                self.games.insert(0, Game(os.path.realpath(last), 'Chosen'))
            self.updating = True
            self.dd_game.set_model(Gtk.StringList.new(
                [g.label() for g in self.games] or ['No Armada II found']))
            self.dd_game.set_selected(next(
                (i for i, g in enumerate(self.games)
                 if last and os.path.realpath(g.path) == os.path.realpath(last)), 0))
            self.dd_game.set_visible(len(self.games) > 1)
            self.updating = False
            self.on_game()

        def on_browse(self):
            d = Gtk.FileDialog(title='The folder holding Armada2.exe', modal=True)
            g = self.game()
            if g:
                d.set_initial_folder(Gio.File.new_for_path(os.path.dirname(g.path)))

            def done(dialog, result):
                try:
                    f = dialog.select_folder_finish(result)
                except GLib.Error:
                    return
                path = f.get_path()
                if not exe_in(path):
                    self.set_status('There is no Armada2.exe in that folder', 'err')
                    return
                remember(game=path)
                self.load_games(select=path)
            d.select_folder(self, None, done)

        def on_game(self):
            if self.updating:
                return
            g = self.game()
            if not g:
                self.facts = None
                self.row_game.set_subtitle('Not found. Choose the folder holding Armada2.exe.')
                self.update_all()
                return
            remember(game=g.path)
            f = self.facts = inspect(g.path)
            inst = f['installed']
            self.row_game.set_subtitle(GLib.markup_escape_text(
                short(g.path) + '\n' + ('Refit ' + inst if inst and inst != '?'
                                        else 'Refit, version unknown' if inst else 'Stock game')))
            self.update_all()

        # ---------------------------------------------------------- releases
        def set_releases(self, rels):
            cur = self.release()
            cur_v = cur.version if cur and self.user_picked else None
            self.releases = rels
            newest = default_release(rels)
            labels = []
            for r in rels:
                tags = ['newest'] if r is newest else []
                if r.prerelease:
                    tags.append('pre-release')
                labels.append(r.version + (f'  ({", ".join(tags)})' if tags else ''))
            self.updating = True
            self.dd_rel.set_model(Gtk.StringList.new(labels or ['No releases']))
            pick = cur_v or (newest.version if newest else None)
            self.dd_rel.set_selected(next((i for i, r in enumerate(rels) if r.version == pick), 0))
            self.updating = False
            self.on_release(user=False)

        def refresh(self):
            if self.busy:
                return
            self.set_busy(True)
            self.set_status(STATUS['fetch'] + '…')
            self.bar.set_fraction(0)
            self.load_games(select=self.game().path if self.game() else None)

            def done(rels):
                self.checked, self.check_error = time.time(), None
                self.set_busy(False)
                self.set_status('')
                self.set_releases(rels)

            def fail(e):
                self.check_error = str(e)
                self.set_busy(False)
                self.set_status('Offline. Showing the releases already downloaded.')
                self.log(f'release check failed: {e}')
                self.set_releases(cached_releases()[0])
            self.in_thread(lambda: merge_cached(fetch_releases()), done, fail)

        def on_release(self, user=True):
            if self.updating:
                return
            if user:
                self.user_picked = True
            r = self.release()
            self.package = None
            if not r:
                self.row_rel.set_subtitle('Nothing to install yet. Check the connection and refresh.')
                self.update_all()
                return
            sub = [r.date] if r.date else []
            if r.prerelease:
                sub.append('pre-release, not yet seen in game')
            if r.is_cached():
                self.read_package(r)
            elif r.url and not self.busy:
                # About a megabyte: fetch it now, to know what it holds.
                sub.append('downloading…')
                self.in_thread(lambda: download(r), lambda _: self.on_release(user=False),
                               lambda e: (self.row_rel.set_subtitle(f'Could not download: {e}'),
                                          self.update_all()))
            if self.check_error and not r.url:
                sub.append('no longer on GitHub')
            self.row_rel.set_subtitle(' · '.join(sub) or ' ')
            self.update_all()

        def read_package(self, r):
            try:
                self.package = Package(r.cached)
            except Exception as e:
                self.row_rel.set_subtitle(f'Unreadable package: {e}')
                self.package = None

        # ---------------------------------------------------------- what to show
        def update_all(self):
            self.update_heads_up()
            self.update_bloom()
            self.update_get_heroic()
            self.update_launcher()
            self.update_banner()
            self.update_buttons()

        def update_heads_up(self):
            while (c := self.heads_up.get_first_child()):
                self.heads_up.remove(c)
            f, p = self.facts, self.package
            if not f:
                return
            notes = []
            if p:
                for l in p.layers:
                    if l['id'] in EXCLUDED or l['id'] == 'bloom':
                        continue
                    yes, why = layer_verdict(l, f)
                    if not yes:
                        notes.append(f'{l["name"]}: {why[len("skipped — "):]}'
                                     if why.startswith('skipped — ') else f'{l["name"]}: {why}')
                    elif 'per vertex' in why:
                        notes.append(f'{l["name"]}: per vertex; per pixel needs d3d8to9')
            for n in notes:
                self.heads_up.append(lab(n, ('heads-up', 'warn'), xalign=0.5, wrap=True,
                                         justify=Gtk.Justification.CENTER, margin_start=8,
                                         margin_end=8))

        def update_bloom(self):
            for r in self.bloom_rows:
                self.row_bloom.remove(r)
            self.bloom_rows = []
            f, g = self.facts, self.game()
            if not f or f['vkbasalt']:
                self.row_bloom.set_visible(False)
                return
            name, steps = vkbasalt_howto()
            self.row_bloom.set_visible(True)
            if g and g.heroic and g.heroic.flatpak:
                r = Adw.ActionRow(title='Heroic runs as a Flatpak',
                                  subtitle='It cannot see a vkBasalt installed this way, so '
                                           'bloom is not supported there yet.')
                self.row_bloom.add_row(r)
                self.bloom_rows.append(r)
            for text, cmd in steps:
                r = Adw.ActionRow(title=GLib.markup_escape_text(text), title_lines=0,
                                  subtitle=GLib.markup_escape_text(cmd), subtitle_lines=0)
                r.add_css_class('property')
                cp = Gtk.Button(icon_name='edit-copy-symbolic', valign=Gtk.Align.CENTER,
                                tooltip_text='Copy')
                cp.add_css_class('flat')
                cp.connect('clicked', lambda _b, c=cmd: self.copy(c))
                r.add_suffix(cp)
                self.row_bloom.add_row(r)
                self.bloom_rows.append(r)
            r = Adw.ActionRow(title=f'Detected: {name}. Refresh and install again afterwards '
                                    'to set bloom up.', title_lines=0)
            r.add_css_class('dim-label')
            self.row_bloom.add_row(r)
            self.bloom_rows.append(r)

        def wanted_env(self):
            f = self.facts
            if not f:
                return {}
            st = f.get('state') or {}
            if st.get('launcher_env') and st.get('version') == (self.package.version
                                                                if self.package else None):
                return st['launcher_env']
            return launch_env(f, self.package)

        def update_launcher(self):
            g, env = self.game(), self.wanted_env()
            self.env_text.set_label('\n'.join(f'{k}={v}' for k, v in env.items()))
            heroic = bool(g and g.heroic)
            self.row_heroic.set_visible(heroic)
            # A game Heroic does not run (Lutris, a Wine prefix): the text to copy.
            self.row_env.set_visible(bool(env) and bool(g) and not heroic)
            if not heroic:
                return
            changes, conflicts = heroic_plan(g.heroic, env)
            if changes is None:
                sub = conflicts[0]
            elif not changes:
                sub = f'Already set in {g.heroic.label()}'
            else:
                sub = f'Written to {g.heroic.label()} on install: {", ".join(c["key"] for c in changes)}'
            if conflicts and changes is not None:
                sub += '. ' + '; '.join(conflicts)
            if heroic_running():
                sub += '. Heroic is running: restart it afterwards'
            self.row_heroic.set_subtitle(GLib.markup_escape_text(sub))

        def update_get_heroic(self):
            for r in self.get_heroic_rows:
                self.row_get_heroic.remove(r)
            self.get_heroic_rows = []
            g = self.game()
            if (g and g.heroic) or heroic_installed():
                self.row_get_heroic.set_visible(False)
                return
            self.row_get_heroic.set_visible(True)
            self.row_get_heroic.set_expanded(True)
            for text, cmd in HEROIC_HOWTO.get(distro_family(), HEROIC_HOWTO['other']):
                r = Adw.ActionRow(title=GLib.markup_escape_text(text), title_lines=0,
                                  subtitle=GLib.markup_escape_text(cmd), subtitle_lines=0)
                r.add_css_class('property')
                cp = Gtk.Button(icon_name='edit-copy-symbolic', valign=Gtk.Align.CENTER,
                                tooltip_text='Copy')
                cp.add_css_class('flat')
                cp.connect('clicked', lambda _b, c=cmd: self.copy(c))
                r.add_suffix(cp)
                self.row_get_heroic.add_row(r)
                self.get_heroic_rows.append(r)
            site = Adw.ActionRow(title='heroicgameslauncher.com', activatable=True)
            site.add_suffix(Gtk.Image(icon_name='adw-external-link-symbolic'))
            site.connect('activated', lambda _r: self.open_uri(HEROIC_SITE))
            self.row_get_heroic.add_row(site)
            self.get_heroic_rows.append(site)
            r = Adw.ActionRow(title='Then sign in to GOG in Heroic, install Star Trek: Armada '
                                    'II, and press F5 here.', title_lines=0)
            r.add_css_class('dim-label')
            self.row_get_heroic.add_row(r)
            self.get_heroic_rows.append(r)

        def update_banner(self):
            p, newer = self.package, None
            for r in self.releases:
                if r.is_cached():
                    try:
                        iv = (Package(r.cached).manifest.get('installer') or {}).get('version')
                    except Exception:
                        continue
                    if iv and vtuple(iv) > vtuple(INSTALLER_VERSION):
                        newer = (iv, r)
                    break
            if p and not p.supported():
                self.banner.set_title(f'Release {p.version} needs a newer installer.')
            elif newer:
                self.banner.set_title(f'A newer installer ({newer[0]}) is available.')
            else:
                self.banner.set_revealed(False)
                return
            r = newer[1] if newer else self.release()
            self.banner_uri = (r.installer_url if r and r.installer_url else
                               r.page if r else RELEASES_PAGE)
            self.banner.set_revealed(True)

        def update_buttons(self):
            g, r, p, f = self.game(), self.release(), self.package, self.facts
            inst = f and f['installed']
            ok = bool(g and r and (p is None or p.supported()) and (p or r.url))
            self.b_install.set_sensitive(not self.busy and ok)
            self.b_uninstall.set_visible(bool(g and inst))
            self.b_uninstall.set_sensitive(not self.busy)
            self.b_launch.set_visible(bool(g and g.heroic and inst))
            self.b_launch.set_sensitive(not self.busy)
            label = 'Install'
            if r and inst:
                label = ('Reinstall' if inst == r.version else 'Install' if inst == '?' else
                         'Upgrade to' if vtuple(r.version) > vtuple(inst) else 'Switch to')
                if label != 'Reinstall' and label != 'Install':
                    label += f' {r.version}'
            self.b_install.set_label(label)
            # The amber goes to what there is to do: Launch once nothing is new to install.
            launch_first = label == 'Reinstall' and self.b_launch.get_visible()
            for b, on in ((self.b_install, not launch_first), (self.b_launch, launch_first)):
                (b.add_css_class if on else b.remove_css_class)('suggested-action')

        # ---------------------------------------------------------- jobs
        def report(self, frac, step, info):
            def ui():
                text = STATUS.get(step, step).format(version=(info or {}).get('version', ''))
                if info and info.get('detail'):
                    text += f' · {info["detail"]}'
                self.set_status(text + ('…' if frac < 1 else ''))
                self.bar.set_fraction(frac)
            GLib.idle_add(ui)

        def job(self):
            return Job(self.report, lambda line: GLib.idle_add(self.log, line))

        def confirm(self, heading, body, action, then, destructive=False):
            d = Adw.AlertDialog(heading=heading, body=body)
            d.add_response('cancel', 'Cancel')
            d.add_response('go', action)
            d.set_response_appearance('go', Adw.ResponseAppearance.DESTRUCTIVE if destructive
                                      else Adw.ResponseAppearance.SUGGESTED)
            d.set_default_response('go')
            d.connect('response', lambda _d, resp: then() if resp == 'go' else None)
            d.present(self)

        def on_install(self):
            g, r = self.game(), self.release()
            if not g or not r or self.busy or not self.b_install.get_sensitive():
                return
            if game_running():
                self.set_status('Armada II is running. Quit the game first.', 'err')
                return
            write = bool(g.heroic)
            if write and heroic_running():
                self.confirm('Heroic is running',
                             'Heroic may write its own copy of the game settings back over '
                             'the launch variables. Close Heroic first, or install now and '
                             'restart Heroic afterwards.', 'Install anyway',
                             lambda: self.start_install(g, r, write))
                return
            self.start_install(g, r, write)

        def start_install(self, g, r, write):
            self.set_busy(True)
            self.sky.set_warp(True)
            self.bar.set_fraction(0)
            self.log(f'== install {r.version} into {g.path}')

            def done(res):
                self.set_busy(False)
                self.sky.set_warp(False)
                msg = f'Refit {res["version"]} installed'
                if write and res['heroic']:
                    msg += '. Heroic launch settings written.'
                elif res['env']:
                    msg += '. Set the launch variables in your launcher.'
                if res['notes']:
                    msg += '\n' + ' · '.join(res['notes'])
                self.set_status(msg, 'ok')
                self.on_game()
            self.in_thread(lambda: self.job().install(r, None, g.path, launcher=write), done)

        def on_launch(self):
            g = self.game()
            if not g or not g.heroic or self.busy:
                return
            if game_running():
                self.set_status('Armada II is already running.')
                return
            if launch_game(g.heroic):
                self.set_status('Starting Armada II through Heroic…', 'ok')
            else:
                self.open_uri(heroic_launch_uri(g.heroic))

        def on_uninstall(self):
            g = self.game()
            if not g or self.busy:
                return
            self.confirm('Uninstall Armada II Refit?',
                         f'Everything the mod put into {short(g.path)} is taken out and the '
                         'stock files put back. Launch settings written to Heroic are taken '
                         'out again.', 'Uninstall', lambda: self.start_uninstall(g),
                         destructive=True)

        def start_uninstall(self, g):
            self.set_busy(True)
            self.bar.set_fraction(0)
            self.log(f'== uninstall from {g.path}')

            def done(_):
                self.set_busy(False)
                self.set_status('Uninstalled. The game is stock again.', 'ok')
                self.on_game()
            self.in_thread(lambda: self.job().uninstall(self.releases, g.path), done)

    provider = Gtk.CssProvider()
    css = (CSS_VARS if (Gtk.get_major_version(), Gtk.get_minor_version()) >= (4, 16) else '') + CSS
    if hasattr(provider, 'load_from_string'):
        provider.load_from_string(css)
    else:
        provider.load_from_data(css.encode(), -1)

    def activate(app):
        Adw.StyleManager.get_default().set_color_scheme(Adw.ColorScheme.FORCE_DARK)
        Gtk.StyleContext.add_provider_for_display(Gdk.Display.get_default(), provider,
                                                  Gtk.STYLE_PROVIDER_PRIORITY_USER)
        win = app.get_active_window() or Window(app)
        win.present()

    app = Adw.Application(application_id='io.github.cedrichaase.Armada2Refit.Installer',
                          flags=Gio.ApplicationFlags.NON_UNIQUE)
    app.connect('activate', activate)
    return app.run([sys.argv[0]])


# ------------------------------------------------------------------ the window on Windows and macOS (Tk)

GOG_PAGE = 'https://www.gog.com/en/game/star_trek_armada_2'
# What the window says when it finds no game; the README quotes these, so they are the
# only copy. Each step is (text, [(link, url)...]).
WINDOWS_HOWTO = dict(
    title='Armada II was not found',
    steps=[('Install Star Trek: Armada II from GOG, with GOG Galaxy or the offline installer '
            'from your GOG library, then press Refresh.',
            [('GOG Galaxy', 'https://www.gog.com/galaxy'), ('The game on GOG', GOG_PAGE)]),
           ('Installed somewhere else? Browse to the folder that holds Armada2.exe.', [])],
    notes=[])
MAC_HOWTO = dict(
    title='On a Mac, Armada II runs in Wine',
    steps=[('CrossOver 26 (paid, CodeWeavers; Armada II rated Runs Great) — create a bottle, '
            'install the GOG offline installer into it, then Refresh.',
            [('CrossOver', 'https://www.codeweavers.com/crossover'),
             ('The game on GOG', GOG_PAGE)]),
           ("Free: Sikarugir (the former Kegworks) wrapper, or Wine from Gcenx's macOS builds.",
            [('Sikarugir', 'https://github.com/Sikarugir-App/Sikarugir'),
             ("Gcenx's Wine builds", 'https://github.com/Gcenx/macOS_Wine_builds')])],
    notes=["Direct3D 8 and 9 run through Wine's own wined3d on a Mac, so leave CrossOver's "
           'graphics backend on its default for this game: D3DMetal, DXVK and DXMT only cover '
           'Direct3D 10 to 12.',
           'Anti-aliasing (MSAA) and bloom are not available on macOS.'])

SKY_TOP, SKY_BOTTOM = (4, 6, 13), (8, 10, 22)      # the gradient, as Sky.build_nebula's


def tk_png(w, h, rows, alpha=True):
    """A PNG of `rows` (bytes of RGBA, or RGB with alpha=False), for tk.PhotoImage: Tk 8.6
    reads PNG with its alpha, and draws it blended over the canvas."""
    import zlib

    def chunk(kind, data):
        return (struct.pack('>I', len(data)) + kind + data +
                struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff))
    raw = b''.join(b'\0' + r for r in rows)
    return (b'\x89PNG\r\n\x1a\n' +
            chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6 if alpha else 2, 0, 0, 0)) +
            chunk(b'IDAT', zlib.compress(raw, 1)) + chunk(b'IEND', b''))


def rounded_rows(w, h, r, fill, edge=None):
    """RGBA rows of a rounded rectangle, antialiased, with a 1 px edge; fill and edge are
    (r, g, b, alpha 0..1). Only the corners are worked out pixel by pixel."""
    import math
    edge = edge or fill
    clamp = lambda v: 0.0 if v < 0 else 1.0 if v > 1 else v

    def px(cf, ce):
        af, ae = fill[3] * cf, edge[3] * ce
        a = ae + af * (1 - ae)
        if a <= 0:
            return b'\0\0\0\0'
        return bytes(min(255, round((edge[i] * ae + fill[i] * af * (1 - ae)) / a))
                     for i in range(3)) + bytes((round(255 * a),))
    r = max(1, min(r, w // 2, h // 2))
    inner, side = px(1, 0), px(0, 1)
    mid = side + inner * (w - 2) + side
    rows = []
    for y in range(h):
        if r <= y < h - r:
            rows.append(mid)
            continue
        cy = r - (y + 0.5) if y < r else (y + 0.5) - (h - r)
        left = []
        for x in range(r):
            d = math.hypot(r - (x + 0.5), cy)
            co, ci = clamp(r - d + 0.5), clamp(r - 1 - d + 0.5)
            left.append(px(ci, co - ci))
        centre = side if y in (0, h - 1) else inner
        rows.append(b''.join(left) + centre * (w - 2 * r) + b''.join(reversed(left)))
    return rows


def glow_rows(size, level):
    """RGBA rows of the progress bar's engine (EngineBar.draw's gradient) at `level`."""
    import math
    stops = ((0.0, (1.0, 0.97, 0.88), 1.0), (0.25, (1.0, 0.72, 0.32), 0.75),
             (1.0, (0.96, 0.57, 0.18), 0.0))
    c, rows = size / 2, []
    for y in range(size):
        row = bytearray()
        for x in range(size):
            t = min(1.0, math.hypot(x + 0.5 - c, y + 0.5 - c) / c)
            (t0, c0, a0), (t1, c1, a1) = (stops[0], stops[1]) if t < 0.25 else (stops[1], stops[2])
            f = (t - t0) / (t1 - t0)
            row += bytes(round(255 * (c0[i] + (c1[i] - c0[i]) * f)) for i in range(3))
            row.append(round(255 * level * (a0 + (a1 - a0) * f)))
        rows.append(bytes(row))
    return rows


def nebula_rgb(buf, t, h_out, w_out):
    """nebula_pixels' premultiplied BGRA over the sky's gradient at opacity t, scaled
    (bilinear; across, it wraps, as the tile does) to w_out x h_out: the bytes of a PPM.
    Composited at 256x160, since the gradient is linear too. Pure Python: run it off
    the main thread (about a third of a second for a 1100x700 tile)."""
    sw, sh = NEBULA_SIZE
    src = []
    for j in range(sh):
        f = (j + 0.5) / sh
        bg = [SKY_TOP[i] + (SKY_BOTTOM[i] - SKY_TOP[i]) * f for i in range(3)]
        row = []
        base = j * sw * 4
        for i in range(sw):
            b, g, r, a = buf[base + 4 * i:base + 4 * i + 4]
            k = 1 - a * t / 255
            row += (bg[0] * k + r * t, bg[1] * k + g * t, bg[2] * k + b * t)
        src.append(row)
    idx = []
    for x in range(w_out):
        u = (x + 0.5) * sw / w_out - 0.5
        i = math.floor(u)
        f = u - i
        a, b = (i % sw) * 3, ((i + 1) % sw) * 3
        idx += ((a, b, f), (a + 1, b + 1, f), (a + 2, b + 2, f))
    hrows = [[r[a] + (r[b] - r[a]) * f for a, b, f in idx] for r in src]
    out = []
    for y in range(h_out):
        v = min(sh - 1.0, max(0.0, (y + 0.5) * sh / h_out - 0.5))
        j = min(sh - 2, int(v))
        f = v - j
        r0, r1 = hrows[j], hrows[j + 1]
        out.append(bytes([int(p + (q - p) * f + 0.5) for p, q in zip(r0, r1)]))
    return b''.join(out)


def run_tk():
    """The window on Windows and macOS: Tk, from the standard library (8.6 and 9). The
    same one screen as run_gui's, but drawn on one Canvas -- the sky, the card, the
    buttons, the pickers -- because a Tk widget is opaque and would cut a box out of the
    sky. The pickers open a menu. Jobs run on a worker thread and reach Tk only through
    a queue polled with after(), since Tk is not thread-safe."""
    import queue
    import random
    import webbrowser
    import tkinter as tk
    from tkinter import filedialog, messagebox
    from tkinter import font as tkfont

    if SYSTEM == 'windows':
        try:        # sharp text on a scaled display; Tk then reports the real DPI
            import ctypes
            ctypes.windll.shcore.SetProcessDpiAwareness(1)
        except Exception:
            pass
    try:
        root = tk.Tk(className='Armada2Refit')
    except tk.TclError as e:
        print(f'The window needs a display ({e}); use the command line: --help.',
              file=sys.stderr)
        return 1
    # S: logical pixels to Tk's. Tk's pixels are points on macOS (Retina is the system's);
    # elsewhere `tk scaling` is the display's DPI / 72, and 96 DPI is 1.
    S = 1.0 if SYSTEM == 'macos' else \
        max(1.0, float(root.tk.call('tk', 'scaling')) / (96 / 72))
    px = lambda v: max(1, round(v * S))
    root.title('Armada II Refit')
    root.geometry(f'{px(760)}x{px(800)}')
    root.minsize(px(720), px(640))
    root.configure(bg='#05070d')

    FG, DIM, FAINT = '#ebe8e2', '#9aa2b4', '#6b7385'
    WARN, ERR, OK = '#f2c46d', '#ff8a7a', '#93d69a'
    SEP = '#232837'
    PANEL = ((16, 20, 31, 0.78), (255, 255, 255, 0.09))
    FIELD = ((0, 0, 0, 0.28), (255, 255, 255, 0.13))
    FIELD_HOT = ((0, 0, 0, 0.28), (245, 168, 73, 0.75))
    AMBER_RGB, AMBER_LIGHT_RGB = (232, 146, 47), (245, 168, 73)
    BUTTONS = {     # kind: ((fill, edge), (hover fill, edge), text)
        'primary': (((*AMBER_RGB, 1.0), (*AMBER_LIGHT_RGB, 1.0)),
                    ((*AMBER_LIGHT_RGB, 1.0), (255, 200, 130, 1.0)), AMBER_INK),
        'secondary': (((255, 255, 255, 0.09), (255, 255, 255, 0.16)),
                      ((255, 255, 255, 0.16), (255, 255, 255, 0.26)), FG),
        'disabled': (((255, 255, 255, 0.05), (255, 255, 255, 0.08)), None, FAINT),
    }

    families = set(tkfont.families(root))
    if SYSTEM == 'windows':
        face = 'Segoe UI'
        heavy = ('Segoe UI Black', 'normal') if 'Segoe UI Black' in families else (face, 'bold')
    elif SYSTEM == 'macos':
        face = tkfont.nametofont('TkDefaultFont').actual('family')   # .AppleSystemUIFont
        if face not in families and 'Helvetica Neue' in families:
            face = 'Helvetica Neue'
        heavy = (face, 'bold')
    else:
        face = next((f for f in ('Noto Sans', 'Cantarell', 'DejaVu Sans') if f in families),
                    tkfont.nametofont('TkDefaultFont').actual('family'))
        heavy = (face, 'bold')
    if face not in families:        # never on the systems above; a font Tk has, then
        face = tkfont.nametofont('TkDefaultFont').actual('family')
        heavy = (face, 'bold') if heavy[0] not in families else heavy
    fonts = {}

    def font(size, weight='normal', family=None, underline=False):
        k = (size, weight, family, underline)
        if k not in fonts:
            fonts[k] = tkfont.Font(root, family=family or face, size=-px(size), weight=weight,
                                   underline=underline)
        return fonts[k]
    F_TITLE = font(44, heavy[1], heavy[0])
    F_TAG, F_ROW, F_SUB = font(14, 'bold'), font(15), font(13)
    F_SMALL, F_BTN, F_LINK = font(12), font(15, 'bold'), font(13, underline=True)
    F_MONO = tkfont.Font(root, family=tkfont.nametofont('TkFixedFont').actual('family'),
                         size=-px(12))
    MOD = 'Command' if SYSTEM == 'macos' else 'Control'
    MOD_NAME = 'Cmd' if SYSTEM == 'macos' else 'Ctrl'

    def photo_png(w, h, rows):
        import base64
        return tk.PhotoImage(master=root, data=base64.b64encode(tk_png(w, h, rows)).decode())

    def photo_rgb(w, h, data):
        try:
            return tk.PhotoImage(master=root, data=b'P6\n%d %d\n255\n' % (w, h) + data,
                                 format='ppm')
        except tk.TclError:     # a Tk that takes no binary PPM as data: the same as a PNG
            rows = [data[y * w * 3:(y + 1) * w * 3] for y in range(h)]
            import base64
            return tk.PhotoImage(master=root,
                                 data=base64.b64encode(tk_png(w, h, rows, alpha=False)).decode())

    shapes = {}

    def shape(w, h, r, colours):
        """A rounded rectangle as a PhotoImage, kept: the card, a button, a field."""
        k = (w, h, r, colours)
        if k not in shapes:
            shapes[k] = photo_png(w, h, rounded_rows(w, h, r, *colours))
        return shapes[k]

    class Sky:
        """Sky (run_gui's) on a Canvas: the gradient, the nebula (faded in once a thread has
        made it, then moved only by whole pixels), three drifting layers of stars, and
        warp streaks while a job runs. At rest a layer is moved as one tag, and only when
        it has travelled a whole pixel, so the canvas redraws a dozen times a second."""
        LAYERS = ((90, 0.55, 3.0), (60, 0.85, 7.0), (28, 1.35, 14.0))   # count, radius, px/s
        CLOUD_SPEED = 1.5
        WARP_SPEED, WARP_EASE, WARP_HOLD = 40.0, 1.2, 3.0
        TINTS = ((255, 255, 255), (191, 217, 255), (255, 224, 191))
        LEVELS = 8

        def __init__(self, cv):
            self.cv = cv
            rnd = random.Random(2399)
            self.stars = [(rnd.random(), rnd.random(), r, rnd.uniform(0.35, 1.0),
                           rnd.uniform(0, 6.3), speed, rnd.choice((0, 0, 0, 1, 2)), layer)
                          for layer, (count, r, speed) in enumerate(self.LAYERS)
                          for _ in range(count)]
            self.w = self.h = 0
            self.grad = None
            self.cloud = None               # nebula_pixels' bytes, once made
            self.tile, self.tile_h, self.tiles, self.tile_gen = None, 0, [], 0
            self.tile_off = None
            self.off = [0, 0, 0]
            self.travel, self.warp, self.warp_to = 0.0, 0.0, 0.0
            self.warp_from, self.warp_off, self.t_last = 0.0, None, None
            self.streaks = False
            self.twinkle_at = 0
            seed = random.SystemRandom().randrange(1 << 30)
            app.in_thread(lambda: nebula_pixels(seed), self.cloud_ready, lambda e: None)

        def colour(self, tint, a):
            t = self.TINTS[tint]
            return '#%02x%02x%02x' % tuple(round(SKY_TOP[i] + (t[i] - SKY_TOP[i]) * a)
                                           for i in range(3))

        def restack(self):
            for tag in ('star', 'neb', 'grad'):
                self.cv.tag_lower(tag)

        def resize(self, w, h):
            if (w, h) == (self.w, self.h):
                return
            self.w, self.h = w, h
            rows = []
            for y in range(h):
                f = (y + 0.5) / h
                rows.append(bytes(round(SKY_TOP[i] + (SKY_BOTTOM[i] - SKY_TOP[i]) * f)
                                  for i in range(3)) * w)
            self.grad = photo_rgb(w, h, b''.join(rows))
            self.cv.delete('grad')
            self.cv.create_image(0, 0, anchor='nw', image=self.grad, tags=('grad',))
            self.build_stars()
            if self.cloud and h != self.tile_h:
                self.make_tiles(fade=self.tile is None)
            self.place_tiles()
            self.restack()

        def build_stars(self):
            cv, w, h = self.cv, self.w, self.h
            cv.delete('star')
            self.off = [0, 0, 0]
            self.streaks = False
            for i, (x, y, r, a, ph, speed, tint, layer) in enumerate(self.stars):
                s = px((1, 2, 2)[layer])
                cx, cy = round(x * w), round(y * h)
                col = self.colour(tint, a * (0.8 if layer == 1 else 1.0))
                for dx in (0, -w):
                    x0, y0 = cx + dx - s // 2, cy - s // 2
                    cv.create_rectangle(x0, y0, x0 + s, y0 + s, fill=col, width=0,
                                        tags=('star', f'L{layer}', f's{i}', 'dot'))
            self.restack()

        # ---------------------------------------------------------- the nebula
        def cloud_ready(self, buf):
            self.cloud = buf
            if self.h:          # else the first resize makes them
                self.make_tiles(fade=True)

        def make_tiles(self, fade):
            self.tile_gen += 1
            gen, h, buf = self.tile_gen, self.h, self.cloud
            self.tile_h = h
            tw = max(1, round(NEBULA_SIZE[0] * h / NEBULA_SIZE[1]))

            def work():
                # The fade's steps at half size (gone in a moment), the last at full.
                for t in ((0.2, 0.4, 0.6, 0.8, 1.0) if fade else (1.0,)):
                    if gen != self.tile_gen:
                        return          # the window changed height again meanwhile
                    k = 1 if t == 1.0 else 2
                    w_, h_ = -(-tw // k), -(-h // k)
                    data = nebula_rgb(buf, t, h_, w_)
                    app.post(self.show_tile, gen, w_, h_, k, data)
                    if t < 1.0:
                        time.sleep(0.12)
            threading.Thread(target=work, daemon=True).start()

        def show_tile(self, gen, w, h, k, data):
            if gen != self.tile_gen:
                return
            img = photo_rgb(w, h, data)
            if k > 1:
                img = img.zoom(k)
            self.tile = img
            self.tile_off = None
            self.place_tiles()

        def place_tiles(self):
            if not self.tile:
                return
            tw = self.tile.width()
            n = self.w // tw + 2
            if len(self.tiles) != n:
                self.cv.delete('neb')
                self.tiles = [self.cv.create_image(0, 0, anchor='nw', image=self.tile,
                                                   tags=('neb',)) for _ in range(n)]
                self.restack()
                self.tile_off = None
            else:
                for t in self.tiles:
                    self.cv.itemconfigure(t, image=self.tile)
            self.move_tiles()

        def move_tiles(self):
            if not self.tiles:
                return
            tw = self.tile.width()
            off = round(self.travel * self.CLOUD_SPEED * S) % tw
            if off != self.tile_off:
                self.tile_off = off
                for k, t in enumerate(self.tiles):
                    self.cv.coords(t, k * tw - off, 0)

        # ---------------------------------------------------------- motion
        def set_warp(self, on):
            t = time.monotonic()
            if on:
                self.warp_to, self.warp_from, self.warp_off = 1.0, t, None
            elif self.warp_to:
                self.warp_off = max(t, self.warp_from + self.WARP_HOLD)

        def speed(self):
            e = self.warp * self.warp * (3 - 2 * self.warp)
            return 1.0 + (self.WARP_SPEED - 1.0) * e

        def busy(self):
            return self.warp > 0.001 or self.warp_to > 0

        def tick(self):
            t = time.monotonic()
            dt = 0.0 if self.t_last is None else min(0.1, t - self.t_last)
            self.t_last = t
            if self.warp_off is not None and t >= self.warp_off:
                self.warp_to, self.warp_off = 0.0, None
            self.warp += (self.warp_to - self.warp) * min(1.0, dt * 3.0 / self.WARP_EASE)
            if abs(self.warp - self.warp_to) < 0.001:
                self.warp = self.warp_to
            mult = self.speed()
            self.travel += dt * mult
            if not self.w:
                return
            moved = False
            for layer, (_, _, speed) in enumerate(self.LAYERS):
                o = round(self.travel * speed * S) % self.w
                if o != self.off[layer]:
                    self.cv.move(f'L{layer}', o - self.off[layer], 0)
                    self.off[layer] = o
                    moved = True
            self.move_tiles()
            if mult >= 1.05:
                self.draw_streaks(dt, mult)
            elif self.streaks:
                self.cv.delete('streak')
                self.cv.itemconfigure('dot', state='normal')
                self.streaks = False
            elif moved:
                self.twinkle(t)

        def twinkle(self, now):
            """A slice of the stars per moved frame: each changes every half second or so."""
            n = len(self.stars)
            for i in range(self.twinkle_at, self.twinkle_at + 24):
                i %= n
                x, y, r, a, ph, speed, tint, layer = self.stars[i]
                tw = 0.75 + 0.25 * math.sin(now * 0.9 + ph)
                lv = round(a * tw * (0.8 if layer == 1 else 1.0) * self.LEVELS) / self.LEVELS
                self.cv.itemconfigure(f's{i}', fill=self.colour(tint, lv))
            self.twinkle_at = (self.twinkle_at + 24) % n

        def draw_streaks(self, dt, mult):
            """Each star a streak behind it, faint at the tail: at least what it covers in a
            frame, so it reads as one motion."""
            cv, w, h = self.cv, self.w, self.h
            if not self.streaks:
                cv.itemconfigure('dot', state='hidden')
                for i, (x, y, r, a, ph, speed, tint, layer) in enumerate(self.stars):
                    # Faint at the tail: a Canvas has no alpha, so the tail is stippled,
                    # which lets the sky through (macOS ignores stipples: solid there).
                    for k, (f, dots) in enumerate(((0.8, 'gray25'), (0.9, 'gray50'),
                                                   (1.0, ''))):
                        cv.create_line(0, 0, 0, 0, fill=self.colour(tint, a * f), stipple=dots,
                                       width=max(1, round(2 * r * S)), capstyle='round',
                                       tags=('star', 'streak', f'w{i}_{k}'))
                self.restack()
                self.streaks = True
            streak = max(dt * 1.6, 0.12)
            for i, (x, y, r, a, ph, speed, tint, layer) in enumerate(self.stars):
                head = (x * w + self.travel * speed * S) % w
                length = max(1.0, speed * S * mult * streak)
                yy = round(y * h)
                for k in range(3):
                    cv.coords(f'w{i}_{k}', head - length * (3 - k) / 3, yy,
                              head - length * (2 - k) / 3, yy)

    class Bar:
        """EngineBar on the Canvas: a 4 px bar that glides to each fraction, with a
        flickering engine glow and a short trail at its tip, fading once full."""
        GLIDE = 4.0

        def __init__(self, cv):
            self.cv = cv
            self.shown = self.target = self.lit = 0.0
            self.geom = None
            self.size = px(30)
            self.glows = [photo_png(self.size, self.size, glow_rows(self.size, (k + 1) / 10))
                          for k in range(10)]
            self.trail = ['#%02x%02x%02x' % tuple(round(27 + (c - 27) * f)
                                                  for c in (255, 214, 150))
                          for f in (0.15, 0.3, 0.45, 0.6, 0.75, 0.9)]

        def place(self, x0, x1, y):
            """Called by the layout, which has just deleted the old items."""
            cv = self.cv
            self.geom = (x0, x1, y)
            h = px(4)
            cv.create_rectangle(x0, y - h // 2, x1, y + h - h // 2, fill='#1b1f2b', width=0,
                                tags=('ui',))
            self.fill = cv.create_rectangle(x0, y - h // 2, x0, y + h - h // 2,
                                            fill=AMBER_LIGHT, width=0, tags=('ui',))
            self.trails = [cv.create_rectangle(0, 0, 0, 0, fill=c, width=0, state='hidden',
                                               tags=('ui',)) for c in self.trail]
            self.glow = cv.create_image(0, y, image=self.glows[0], state='hidden', tags=('ui',))
            self.draw()

        def set(self, f):
            if f < self.shown:
                self.shown = f
            self.target = f

        def busy(self):
            return abs(self.target - self.shown) > 1e-4 or self.lit > 0 or \
                0.0 < self.target < 1.0

        def tick(self, dt):
            lit_to = 1.0 if 0.0 < self.target < 1.0 else 0.0
            self.shown += (self.target - self.shown) * min(1.0, dt * self.GLIDE)
            if abs(self.target - self.shown) < 1e-4:
                self.shown = self.target
            self.lit += (lit_to - self.lit) * min(1.0, dt * (5.0 if lit_to else 1.5))
            if self.lit < 0.02 and not lit_to:
                self.lit = 0.0
            self.draw()

        def draw(self):
            if not self.geom:
                return
            cv, (x0, x1, y) = self.cv, self.geom
            h = px(4)
            x = x0 + (x1 - x0) * self.shown
            cv.coords(self.fill, x0, y - h // 2, x, y + h - h // 2)
            cv.itemconfigure(self.fill, state='normal' if x > x0 + 0.5 else 'hidden')
            on = self.lit > 0 and self.shown > 0
            if not on:
                cv.itemconfigure(self.glow, state='hidden')
                for t in self.trails:
                    cv.itemconfigure(t, state='hidden')
                return
            t = time.monotonic()
            flick = 0.85 + 0.1 * math.sin(t * 23) + 0.05 * math.sin(t * 61)
            lv = max(0, min(9, round(self.lit * flick * 10) - 1))
            cv.itemconfigure(self.glow, image=self.glows[lv], state='normal')
            cv.coords(self.glow, x, y)
            seg = min(x - x0, px(80)) / len(self.trails)
            for k, item in enumerate(self.trails):
                a = x - seg * (len(self.trails) - k)
                cv.coords(item, a, y - h // 2, a + seg, y + h - h // 2)
                cv.itemconfigure(item, state='normal' if self.lit > 0.3 else 'hidden')

    class App:
        def __init__(self):
            self.q = queue.Queue()
            self.games, self.gi, self.releases, self.ri = [], 0, [], 0
            self.package, self.facts = None, None
            self.busy, self.user_picked, self.check_error = False, False, None
            self.rel_note = None
            self.status, self.status_kind = '', ''
            self.log_lines, self.logwin = [], None
            self.layout_pending = None
            self.size = self.view = None

        def start(self):
            root.report_callback_exception = self.bug
            self.cv = tk.Canvas(root, highlightthickness=0, bd=0, bg='#05070d')
            self.cv.pack(fill='both', expand=True)
            self.sky = Sky(self.cv)
            self.bar = Bar(self.cv)
            self.cv.bind('<Configure>', self.on_configure)
            root.bind('<F5>', lambda e: self.refresh())
            for k in ('Return', 'KP_Enter'):
                root.bind(f'<{MOD}-{k}>', lambda e: self.on_install())
            for k in ('l', 'L'):
                root.bind(f'<{MOD}-{k}>', lambda e: self.show_log())
            self.t_last = time.monotonic()
            self.poll()
            self.frame()
            self.load_games()
            self.refresh()

        # ---------------------------------------------------------- threads
        def post(self, fn, *args):
            """From any thread: run fn(*args) on Tk's."""
            self.q.put((fn, args))

        def poll(self):
            try:
                while True:
                    fn, args = self.q.get_nowait()
                    try:
                        fn(*args)
                    except Exception:   # a bug in a callback must not stop the loop
                        self.bug()
            except queue.Empty:
                pass
            root.after(40, self.poll)

        def bug(self, *_exc):
            """A Python error in the window's own code: into the log, since a frozen
            program has no console to print it on (sys.stderr is None there)."""
            import traceback
            self.log(traceback.format_exc().rstrip())
            self.status, self.status_kind = 'Something went wrong: see the log (' + \
                MOD_NAME + '+L)', 'err'
            try:
                self.update_all()
            except Exception:
                pass

        def in_thread(self, fn, done=None, fail=None):
            def run():
                try:
                    r = fn()
                except Exception as e:
                    self.post(fail or self.failed, e)
                else:
                    if done:
                        self.post(done, r)
            threading.Thread(target=run, daemon=True).start()

        def frame(self):
            t = time.monotonic()
            dt, self.t_last = min(0.1, t - self.t_last), t
            active = False
            try:
                self.sky.tick()
                active = self.bar.busy()
                if active:
                    self.bar.tick(dt)
            except Exception:
                self.bug()
            # About 30 frames a second; at warp and while the bar moves, about 60.
            root.after(16 if self.sky.busy() or active else 33, self.frame)

        # ---------------------------------------------------------- small things
        def log(self, line):
            self.log_lines.append(line)
            if self.logwin:
                t = self.logwin.text
                t.configure(state='normal')
                t.insert('end', line + '\n')
                t.configure(state='disabled')
                t.see('end')

        def show_log(self):
            if self.logwin and self.logwin.winfo_exists():
                self.logwin.lift()
                return
            w = tk.Toplevel(root, bg='#0b0e16')
            w.title('Installation log')
            w.geometry(f'{px(680)}x{px(440)}')
            bar = tk.Frame(w, bg='#0b0e16')
            bar.pack(side='bottom', fill='x')
            text = tk.Text(w, bg='#0b0e16', fg=FG, insertbackground=FG, font=F_MONO, wrap='word',
                           relief='flat', bd=0, padx=px(10), pady=px(8),
                           selectbackground=AMBER, selectforeground=AMBER_INK,
                           highlightthickness=0)
            sb = tk.Scrollbar(w, command=text.yview)
            text.configure(yscrollcommand=sb.set)
            sb.pack(side='right', fill='y')
            text.pack(fill='both', expand=True)
            text.insert('end', ''.join(l + '\n' for l in self.log_lines) or '')
            text.configure(state='disabled')
            text.see('end')

            def copy_all():
                self.copy('\n'.join(self.log_lines))
            tk.Button(bar, text='Copy all', command=copy_all, font=F_SUB, bg='#1b2030', fg=FG,
                      activebackground=AMBER, activeforeground=AMBER_INK, relief='flat',
                      bd=0, padx=px(12), pady=px(4), highlightthickness=0).pack(
                side='right', padx=px(8), pady=px(6))
            # Disabled text still selects; make the copy key work on it everywhere.
            text.bind('<1>', lambda e: text.focus_set())
            text.bind(f'<{MOD}-a>', lambda e: (text.tag_add('sel', '1.0', 'end'), 'break')[1])
            w.text = text
            w.bind('<Escape>', lambda e: w.destroy())
            self.logwin = w

        def copy(self, text):
            root.clipboard_clear()
            root.clipboard_append(text)
            self.set_status('Copied to the clipboard', 'ok')

        def set_status(self, text, kind=''):
            """Only the status changes: laid out again from what update_all worked out."""
            self.status, self.status_kind = text, kind
            if self.size:
                self.layout()

        def failed(self, e):
            self.busy = False
            self.sky.set_warp(False)
            self.bar.set(0)
            self.log(f'error: {e}')
            self.set_status(f'Failed: {e}', 'err')
            messagebox.showerror('Armada II Refit', str(e), parent=root)

        def game(self):
            return self.games[self.gi] if 0 <= self.gi < len(self.games) else None

        def release(self):
            return self.releases[self.ri] if 0 <= self.ri < len(self.releases) else None

        # ---------------------------------------------------------- the game
        def load_games(self, select=None):
            self.games = find_games()
            last = select or settings().get('game')
            if last and exe_in(last) and not any(
                    os.path.realpath(g.path) == os.path.realpath(last) for g in self.games):
                self.games.insert(0, Game(os.path.realpath(last), 'Chosen', prefix=(
                    prefix_of(last) if SYSTEM == 'macos' else None)))
            self.gi = next((i for i, g in enumerate(self.games)
                            if last and os.path.realpath(g.path) == os.path.realpath(last)), 0)
            self.on_game()

        def on_browse(self):
            if self.busy:
                return
            g = self.game()
            path = filedialog.askdirectory(parent=root, mustexist=True,
                                           title='The folder that holds Armada2.exe',
                                           initialdir=os.path.dirname(g.path) if g else HOME)
            if not path:
                return
            path = os.path.normpath(path)
            if not exe_in(path):
                self.set_status('There is no Armada2.exe in that folder', 'err')
                return
            remember(game=path)
            self.load_games(select=path)

        def pick_game(self, i):
            self.gi = i
            self.on_game()

        def on_game(self):
            g = self.game()
            self.facts = None
            if g:
                remember(game=g.path)
                self.facts = inspect(g.path)
            self.update_all()

        # ---------------------------------------------------------- releases
        def set_releases(self, rels):
            cur = self.release()
            cur_v = cur.version if cur and self.user_picked else None
            self.releases = rels
            newest = default_release(rels)
            pick = cur_v or (newest.version if newest else None)
            self.ri = next((i for i, r in enumerate(rels) if r.version == pick), 0)
            self.on_release(user=False)

        def release_label(self, r):
            tags = ['newest'] if r is default_release(self.releases) else []
            if r.prerelease:
                tags.append('pre-release')
            return r.version + (f'  ({", ".join(tags)})' if tags else '')

        def refresh(self):
            if self.busy:
                return
            self.busy = True
            self.status, self.status_kind = STATUS['fetch'] + '…', ''
            self.bar.set(0)
            g = self.game()
            self.load_games(select=g.path if g else None)

            def done(rels):
                self.check_error = None
                self.busy = False
                self.status = ''
                self.set_releases(rels)

            def fail(e):
                self.check_error = str(e)
                self.busy = False
                self.status, self.status_kind = \
                    'Offline. Showing the releases already downloaded.', ''
                self.log(f'release check failed: {e}')
                self.set_releases(cached_releases()[0])
            self.in_thread(lambda: merge_cached(fetch_releases()), done, fail)

        def pick_release(self, i):
            self.ri = i
            self.on_release(user=True)

        def on_release(self, user=True):
            if user:
                self.user_picked = True
            r = self.release()
            self.package, self.rel_note = None, None
            if r and r.is_cached():
                try:
                    self.package = Package(r.cached)
                except Exception as e:
                    self.rel_note = f'Unreadable package: {e}'
            elif r and r.url and not self.busy:
                # About a megabyte: fetch it now, to know what it holds.
                self.rel_note = 'downloading…'

                def fail(e):
                    if self.release() is r:
                        self.rel_note = f'Could not download: {e}'
                        self.update_all()
                self.in_thread(lambda: download(r),
                               lambda _: self.release() is r and self.on_release(user=False),
                               fail)
            self.update_all()

        # ---------------------------------------------------------- what to show
        def wanted_env(self):
            f = self.facts
            if not f:
                return {}
            st = f.get('state') or {}
            if st.get('launcher_env') and st.get('version') == (self.package.version
                                                                if self.package else None):
                return st['launcher_env']
            return launch_env(f, self.package)

        def launcher_info(self):
            """(title, text, text to copy or None, kind) for the launch-settings row."""
            g = self.game()
            if not g:
                return None
            if SYSTEM == 'windows':
                return ('Launch settings', 'None needed: on Windows the DLLs in the game '
                        'folder load first.', None, '')
            env = self.wanted_env()
            if g.prefix:
                changes, conflicts = prefix_plan(g.prefix, env)
                kind = ''
                if changes is None:
                    text, kind = conflicts[0], 'warn'
                elif changes and wineserver_running():
                    text, kind = f'{g.prefix.label()}: Wine is running in it, quit it first', 'warn'
                elif not changes:
                    text = f'DLL overrides already set in {g.prefix.label()}'
                else:
                    text = (f'Written into {g.prefix.label()} on install: DLL overrides for '
                            + ', '.join(f'{c["key"]} ({c["new"]})' for c in changes))
                if conflicts and changes is not None:
                    text += '. ' + '; '.join(conflicts)
                if not prefix_wine(g.prefix):
                    text += '. No Wine found for it: start the game from its own launcher'
                return ('Wine prefix', text, None, kind)
            if g.heroic:
                changes, conflicts = heroic_plan(g.heroic, env)
                text = conflicts[0] if changes is None else \
                    f'Already set in {g.heroic.label()}' if not changes else \
                    f'Written to {g.heroic.label()} on install: ' + \
                    ', '.join(c['key'] for c in changes)
                return ('Heroic launch settings', text, None, '')
            if env:
                return ('Launch variables', 'No Wine prefix found for this game. Set these '
                        'where it is started:', '\n'.join(f'{k}={v}' for k, v in env.items()), '')
            return None

        def layer_lines(self):
            """(installs, [what will not]) for the chosen release on this game."""
            f, p = self.facts, self.package
            if not f or not p:
                return None, []
            yes, notes = [], []
            for l in p.layers:
                if l['id'] in EXCLUDED or (l['id'] == 'bloom' and SYSTEM == 'linux'):
                    continue
                ok, why = layer_verdict(l, f)
                if ok:
                    yes.append(l['name'])
                    if 'per vertex' in why:
                        notes.append(f'{l["name"]}: per vertex; per pixel needs d3d8to9')
                else:
                    why = why[len('skipped — '):] if why.startswith('skipped — ') else why
                    notes.append(f'{l["name"]}: {why}')
            return ('Goes in: ' + ', '.join(yes)) if yes else None, notes

        def banner(self):
            p, newer = self.package, None
            for r in self.releases:
                if r.is_cached():
                    try:
                        iv = (Package(r.cached).manifest.get('installer') or {}).get('version')
                    except Exception:
                        continue
                    if iv and vtuple(iv) > vtuple(INSTALLER_VERSION):
                        newer = (iv, r)
                    break
            if p and not p.supported():
                r = newer[1] if newer else self.release()
                return f'Release {p.version} needs a newer installer.', \
                    (r.page if r else RELEASES_PAGE)
            if newer:
                return f'A newer installer ({newer[0]}) is available.', newer[1].page
            return None

        def actions(self):
            """(install label, install primary?, can install, launch shown, uninstall shown)"""
            g, r, p, f = self.game(), self.release(), self.package, self.facts
            inst = f and f['installed']
            ok = bool(g and r and (p is None or p.supported()) and (p or r.url))
            label = 'Install'
            if r and inst:
                label = ('Reinstall' if inst == r.version else 'Install' if inst == '?' else
                         'Upgrade to' if vtuple(r.version) > vtuple(inst) else 'Switch to')
                if label not in ('Reinstall', 'Install'):
                    label += f' {r.version}'
            launch = bool(g and inst and (SYSTEM == 'windows' or g.prefix))
            primary = not (label == 'Reinstall' and launch)
            return label, primary, ok and not self.busy, launch, bool(g and inst)

        # ---------------------------------------------------------- layout
        def on_configure(self, e):
            self.size = (e.width, e.height)
            if self.layout_pending:
                root.after_cancel(self.layout_pending)
            self.layout_pending = root.after(60, self.relayout)

        def relayout(self):
            self.layout_pending = None
            w, h = self.size
            self.sky.resize(w, h)
            self.layout()

        def update_all(self):
            """Work out what the window shows (which may read the prefix's registry and
            list processes), then lay it out."""
            self.view = dict(banner=self.banner(), info=self.launcher_info(),
                             lines=self.layer_lines())
            if self.size:
                self.layout()

        def layout(self):
            cv = self.cv
            cv.delete('ui')
            cv.configure(cursor='')
            if len(shapes) > 48:        # sizes the window had; nothing shows them now
                shapes.clear()
            if self.view is None:
                self.update_all()
                return
            W, H = self.size
            cw = min(px(540), W - px(48))
            x0, x1, cx = (W - cw) // 2, (W + cw) // 2, W // 2
            pad = px(20)
            y = 0
            uid = [0]

            def tag():
                uid[0] += 1
                return f'u{uid[0]}'

            def text(x, y, s, f, fill=FG, anchor='nw', width=0, justify='left', tags=()):
                return cv.create_text(x, y, text=s, font=f, fill=fill, anchor=anchor,
                                      width=width, justify=justify, tags=('ui',) + tuple(tags))

            def bottom(item):
                return cv.bbox(item)[3] if cv.bbox(item) else 0

            def hover(t, on_img=None, off_img=None, img_item=None, txt=None, on_fill=None,
                      off_fill=None):
                def enter(_e):
                    cv.configure(cursor='hand2')
                    if img_item and on_img:
                        cv.itemconfigure(img_item, image=on_img)
                    if txt and on_fill:
                        cv.itemconfigure(txt, fill=on_fill)

                def leave(_e):
                    cv.configure(cursor='')
                    if img_item and off_img:
                        cv.itemconfigure(img_item, image=off_img)
                    if txt and off_fill:
                        cv.itemconfigure(txt, fill=off_fill)
                cv.tag_bind(t, '<Enter>', enter)
                cv.tag_bind(t, '<Leave>', leave)

            def button(x, y, w, h, label, kind, cmd, enabled=True, f=F_BTN):
                kind = kind if enabled else 'disabled'
                (norm, hot, ink) = BUTTONS[kind]
                t = tag()
                img = cv.create_image(x, y, anchor='nw', image=shape(w, h, h // 2, norm),
                                      tags=('ui', t))
                txt = text(x + w // 2, y + h // 2, label, f, ink, anchor='center', tags=(t,))
                if enabled:
                    cv.tag_bind(t, '<Button-1>', lambda e: cmd())
                    hover(t, shape(w, h, h // 2, hot), shape(w, h, h // 2, norm), img)
                return txt

            def link(x, y, label, cmd, anchor='nw', f=F_LINK, fill=AMBER_LIGHT):
                t = tag()
                item = text(x, y, label, f, fill, anchor=anchor, tags=(t,))
                cv.tag_bind(t, '<Button-1>', lambda e: cmd())
                hover(t, txt=item, on_fill='#ffd49a', off_fill=fill)
                return item

            def field(x, y, w, h, label, cmd, enabled):
                t = tag()
                img = cv.create_image(x, y, anchor='nw', image=shape(w, h, px(9), FIELD),
                                      tags=('ui', t))
                room = w - px(40)
                s = label
                while s and F_SUB.measure(s) > room:
                    s = s[:-2] + '…' if len(s) > 2 else ''
                text(x + px(12), y + h // 2, s, F_SUB, FG if enabled else FAINT, anchor='w',
                     tags=(t,))
                text(x + w - px(14), y + h // 2, '▾', F_SUB, AMBER_LIGHT if enabled else FAINT,
                     anchor='center', tags=(t,))
                if enabled:
                    # Where it is now: the layout moves everything once it is built.
                    cv.tag_bind(t, '<Button-1>', lambda e: cmd(cv.bbox(img)[0], cv.bbox(img)[3]))
                    hover(t, shape(w, h, px(9), FIELD_HOT), shape(w, h, px(9), FIELD), img)

            def popup(labels, current, pick):
                def show(x, y):
                    m = tk.Menu(root, tearoff=0, bg='#141927', fg=FG, activebackground=AMBER,
                                activeforeground=AMBER_INK, selectcolor=AMBER_LIGHT,
                                relief='flat', bd=px(1), font=F_SUB)
                    var = tk.IntVar(m, current)
                    for i, l in enumerate(labels):
                        m.add_radiobutton(label=l, variable=var, value=i,
                                          command=lambda i=i: pick(i))
                    m.tk_popup(cv.winfo_rootx() + x, cv.winfo_rooty() + y + px(2))
                    m.grab_release()
                return show

            # The banner: a newer installer.
            b = self.view['banner']
            if b:
                bh = px(40)
                t1 = text(x0 + px(16), y + bh // 2, b[0], F_SUB, FG, anchor='w')
                link(x1 - px(16), y + bh // 2, 'Download', lambda u=b[1]: webbrowser.open(u),
                     anchor='e')
                img = cv.create_image(x0, y, anchor='nw', image=shape(
                    cw, bh, px(12), ((232, 146, 47, 0.16), (245, 168, 73, 0.55))), tags=('ui',))
                cv.tag_lower(img, t1)
                y += bh + px(18)

            # The title.
            title = text(cx, y, 'ARMADA II', F_TITLE, '#f3f1ec', anchor='n')
            for dx, dy in ((-2, 0), (2, 0), (0, -2), (0, 2)):
                halo = text(cx + dx, y + dy, 'ARMADA II', F_TITLE, '#16213a', anchor='n')
                cv.tag_lower(halo, title)
            y = bottom(title) - px(2)
            track = F_TAG.measure('M') * 0.9
            letters = 'REFIT'
            tw = sum(F_TAG.measure(c) for c in letters) + track * (len(letters) - 1)
            lx = cx - tw / 2
            for c in letters:
                text(lx, y, c, F_TAG, AMBER_LIGHT)
                lx += F_TAG.measure(c) + track
            y += F_TAG.metrics('linespace') + px(22)

            # The card.
            card_top = y
            card = cv.create_image(x0, y, anchor='nw', tags=('ui',))
            ix0, ix1 = x0 + pad, x1 - pad
            g, r = self.game(), self.release()
            fh = px(34)
            seps = []

            def row(y, title_s, sub, right_w, sub_fill=DIM):
                """A row's title and subtitle, centred on the controls at its right: returns
                (its bottom, its middle)."""
                t_ = tag()
                tw_ = ix1 - ix0 - right_w - (px(14) if right_w else 0)
                t = text(ix0, y, title_s, F_ROW, FG, width=tw_, tags=(t_,))
                end = bottom(t)
                if sub:
                    s = text(ix0, end + px(3), sub, F_SUB, sub_fill, width=tw_, tags=(t_,))
                    end = bottom(s)
                block = end - y
                hgt = max(block, fh) + px(28)
                cv.move(t_, 0, (hgt - block) // 2)
                return y + hgt, y + hgt // 2

            # Game.
            if g:
                inst = self.facts['installed'] if self.facts else None
                state = ('Refit ' + inst if inst and inst != '?' else
                         'Refit, version unknown' if inst else 'Stock game')
                where = short(g.path)
                drive = os.path.join(g.prefix.root, 'drive_c') if g.prefix else None
                if drive and os.path.realpath(g.path).startswith(os.path.realpath(drive) + os.sep):
                    # In a prefix, the path the game itself sees.
                    rel = os.path.relpath(os.path.realpath(g.path), os.path.realpath(drive))
                    where = f'{g.prefix.label()} · C:\\' + rel.replace('/', '\\')
                sub = f'{where}\n{state}'
                if len(self.games) > 1:
                    sub = f'{g.source}\n' + sub
            else:
                sub = 'Not found. Install it, or choose the folder that holds Armada2.exe.'
            bw = F_SUB.measure('Browse…') + px(28)
            pw = px(210) if len(self.games) > 1 else 0
            right = bw + (pw + px(8) if pw else 0)
            y_end, yc = row(y, 'Game', sub, right)
            if pw:
                field(ix1 - right, yc - fh // 2, pw, fh, g.label() if g else '',
                      popup([x.label() for x in self.games], self.gi, self.pick_game),
                      not self.busy)
            button(ix1 - bw, yc - fh // 2, bw, fh, 'Browse…', 'secondary', self.on_browse,
                   not self.busy, F_SUB)
            y = y_end

            # No game: how to get one.
            if not g and SYSTEM != 'linux':
                how = MAC_HOWTO if SYSTEM == 'macos' else WINDOWS_HOWTO
                seps.append(y)
                yy = y + px(14)
                t = text(ix0, yy, how['title'], F_ROW, AMBER_LIGHT, width=ix1 - ix0)
                yy = bottom(t) + px(8)
                for n, (s, links) in enumerate(how['steps'], 1):
                    text(ix0, yy, f'{n}.', F_SUB, DIM)
                    t = text(ix0 + px(20), yy, s, F_SUB, FG, width=ix1 - ix0 - px(20))
                    yy = bottom(t) + px(4)
                    lx = ix0 + px(20)
                    for label, url in links:
                        item = link(lx, yy, label + ' ↗', lambda u=url: webbrowser.open(u))
                        lx = cv.bbox(item)[2] + px(16)
                    if links:
                        yy = bottom(item) + px(4)
                    yy += px(6)
                for n in how['notes']:
                    t = text(ix0, yy, n, F_SMALL, DIM, width=ix1 - ix0)
                    yy = bottom(t) + px(6)
                y = yy + px(8)

            # Version.
            seps.append(y)
            if r:
                bits = [r.date] if r.date else []
                if r.prerelease:
                    bits.append('pre-release, not yet seen in game')
                if self.rel_note:
                    bits.append(self.rel_note)
                if self.check_error and not r.url:
                    bits.append('no longer on GitHub')
                sub = ' · '.join(bits)
            else:
                sub = self.rel_note or ('Checking…' if self.busy else
                                        'Nothing to install yet. Check the connection and refresh.')
            rw = px(210)
            y_end, yc = row(y, 'Version', sub, rw)
            field(ix1 - rw, yc - fh // 2, rw, fh, self.release_label(r) if r else 'No releases',
                  popup([self.release_label(x) for x in self.releases], self.ri,
                        self.pick_release), bool(self.releases) and not self.busy)
            y = y_end

            # Launch settings.
            info = self.view['info']
            if info:
                seps.append(y)
                ttl, sub, copy_s, kind = info
                cw_ = F_LINK.measure('Copy') + px(8) if copy_s else 0
                y_end, yc = row(y, ttl, sub, cw_, WARN if kind == 'warn' else DIM)
                if copy_s:
                    t = text(ix0, y_end - px(6), copy_s, F_MONO, '#d9d4c7', width=ix1 - ix0)
                    link(ix1, y + px(14), 'Copy', lambda s=copy_s: self.copy(s), anchor='ne')
                    y_end = bottom(t) + px(14)
                y = y_end

            ch = y - card_top
            cv.itemconfigure(card, image=shape(cw, ch, px(18), PANEL))
            for sy in seps:
                cv.create_line(x0 + 1, sy, x1 - 1, sy, fill=SEP, tags=('ui',))
            y += px(14)

            # What goes in, and what will not.
            goes, notes = self.view['lines']
            if goes:
                t = text(cx, y, goes, F_SMALL, DIM, anchor='n', width=cw, justify='center')
                y = bottom(t) + px(3)
            for n in notes:
                t = text(cx, y, n, F_SMALL, WARN, anchor='n', width=cw, justify='center')
                y = bottom(t) + px(1)
            y += px(18)

            # The buttons.
            label, primary, can, launch, uninst = self.actions()
            bw, bh, gap = px(200), px(48), px(14)
            n = 2 if launch else 1
            bx = cx - (n * bw + (n - 1) * gap) // 2
            button(bx, y, bw, bh, label, 'primary' if primary else 'secondary',
                   self.on_install, can)
            if launch:
                button(bx + bw + gap, y, bw, bh, '▶  Launch game',
                       'secondary' if primary else 'primary', self.on_launch, not self.busy)
            y += bh + px(14)
            links = ([('Uninstall', self.on_uninstall)] if uninst else []) + \
                [('Refresh', self.refresh), ('Log', self.show_log),
                 ('Releases', lambda: webbrowser.open(RELEASES_PAGE))]
            dot = '   ·   '
            widths = [F_SUB.measure(s) for s, _ in links]
            total = sum(widths) + F_SUB.measure(dot) * (len(links) - 1)
            lx = cx - total / 2
            for k, ((s, fn), w_) in enumerate(zip(links, widths)):
                if self.busy and s in ('Uninstall', 'Refresh'):
                    text(lx, y, s, F_SUB, FAINT)
                else:
                    link(lx, y, s, fn, f=F_SUB, fill=DIM)
                lx += w_
                if k < len(links) - 1:
                    text(lx, y, dot, F_SUB, FAINT)
                    lx += F_SUB.measure(dot)
            y += F_SUB.metrics('linespace') + px(22)

            # Progress and status.
            self.bar.place(x0 + px(40), x1 - px(40), y)
            y += px(14)
            colour = {'err': ERR, 'ok': OK, 'warn': WARN}.get(self.status_kind, DIM)
            t = text(cx, y, self.status or ' ', F_SUB, colour, anchor='n', width=cw,
                     justify='center')
            y = bottom(t)

            # Centred in the window, or from the top when it does not fit.
            dy = max(px(16), (H - y) // 2)
            cv.move('ui', 0, dy)
            self.bar.geom = (self.bar.geom[0], self.bar.geom[1], self.bar.geom[2] + dy)

        # ---------------------------------------------------------- jobs
        def report(self, frac, step, info):
            def ui():
                text = STATUS.get(step, step).format(version=(info or {}).get('version', ''))
                if info and info.get('detail'):
                    text += f' · {info["detail"]}'
                self.bar.set(frac)
                self.set_status(text + ('…' if frac < 1 else ''))
            self.post(ui)

        def job(self):
            return Job(self.report, lambda line: self.post(self.log, line))

        def on_install(self):
            g, r = self.game(), self.release()
            if not g or not r or self.busy or not self.actions()[2]:
                return
            if game_running():
                self.set_status('Armada II is running. Quit the game first.', 'err')
                return
            if g.prefix and wineserver_running():
                changes, _ = prefix_plan(g.prefix, self.wanted_env())
                if changes and not messagebox.askyesno(
                        'Wine is running',
                        f'Wine is running, so the DLL overrides cannot be written into '
                        f'{g.prefix.label()} now. Install anyway (the overrides will not be '
                        'written)?\n\nOr quit every program in Wine first, then install.',
                        icon='warning', parent=root):
                    return
            self.busy = True
            self.sky.set_warp(True)
            self.bar.set(0)
            self.status, self.status_kind = '', ''
            self.log(f'== install {r.version} into {g.path}')
            self.update_all()

            def done(res):
                self.busy = False
                self.sky.set_warp(False)
                msg = f'Refit {res["version"]} installed'
                if res['prefix'] or res['heroic']:
                    msg += '. Launch settings written.'
                elif res['env'] and SYSTEM != 'windows':
                    msg += '. Set the launch variables where the game is started.'
                if res['notes']:
                    msg += '\n' + ' · '.join(res['notes'])
                self.status, self.status_kind = msg, 'ok'
                self.on_game()
            self.in_thread(lambda: self.job().install(r, None, g.path), done)

        def on_launch(self):
            g = self.game()
            if not g or self.busy:
                return
            if game_running():
                self.set_status('Armada II is already running.')
                return
            env = ((self.facts or {}).get('state') or {}).get('launcher_env') or self.wanted_env()
            if launch_game_native(g, env):
                self.set_status('Starting Armada II…', 'ok')
            elif g.prefix:
                self.set_status(f'Could not start the game: no Wine found for '
                                f'{g.prefix.label()}. Start it from there.', 'err')
            else:
                self.set_status('Could not start the game.', 'err')

        def on_uninstall(self):
            g = self.game()
            if not g or self.busy:
                return
            more = ' DLL overrides written into the Wine prefix are taken out again.' \
                if g.prefix else ''
            if not messagebox.askyesno(
                    'Uninstall Armada II Refit?',
                    f'Everything the mod put into {short(g.path)} is taken out and the stock '
                    f'files are put back.{more}', icon='warning', parent=root):
                return
            self.busy = True
            self.bar.set(0)
            self.log(f'== uninstall from {g.path}')
            self.update_all()

            def done(_):
                self.busy = False
                self.status, self.status_kind = 'Uninstalled. The game is stock again.', 'ok'
                self.on_game()
            self.in_thread(lambda: self.job().uninstall(self.releases, g.path), done)

    app = App()
    app.start()
    root.mainloop()
    return 0


def run_python(argv):
    """Run argv[0], a .py file, with argv[1:] as python would: the frozen .exe and .app
    stand in for Python for the release scripts (A2_PYTHON), so the SOD rewrites need
    none on the player's machine. They use only the standard library."""
    import runpy
    sys.argv = list(argv)
    sys.path.insert(0, os.path.dirname(os.path.abspath(argv[0])))
    try:
        runpy.run_path(argv[0], run_name='__main__')
    except SystemExit as e:
        return e.code if isinstance(e.code, int) else (0 if e.code is None else 1)
    except BaseException:
        # A windowed .exe would put an uncaught exception in a dialog and stall the
        # install script waiting on it: the traceback goes to stderr instead.
        import traceback
        traceback.print_exc()
        return 1
    return 0


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1].endswith('.py') and os.path.isfile(sys.argv[1]):
        sys.exit(run_python(sys.argv[1:]))
    if len(sys.argv) > 1 and not (SYSTEM == 'macos' and sys.argv[1].startswith('-psn_')):
        sys.exit(cli(sys.argv[1:]))
    # A2_INSTALLER_TK=1: the Windows and macOS window on Linux, for working on it.
    if SYSTEM != 'linux' or os.environ.get('A2_INSTALLER_TK') == '1':
        sys.exit(run_tk())
    try:
        sys.exit(run_gui())
    except (ImportError, ValueError) as e:
        print(f'The window needs PyGObject with GTK 4 and libadwaita ({e}).\n'
              'Use the AppImage, which carries them, or the command line: --help.',
              file=sys.stderr)
        sys.exit(1)
