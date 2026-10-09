#!/usr/bin/env python3
"""Armada II Refit -- the graphical installer, for Linux (Wine/Proton, GOG release).

    python3 armada2-refit-installer.py            the window
    python3 armada2-refit-installer.py --list     what it finds: games, releases
    python3 armada2-refit-installer.py --install [VERSION] [--game DIR] [--no-launcher]
    python3 armada2-refit-installer.py --package ZIP --game DIR [--no-launcher]
    python3 armada2-refit-installer.py --uninstall [--game DIR]
    python3 armada2-refit-installer.py --selftest

It finds the game (Heroic, Lutris, Bottles, Wine prefixes, the usual folders), lists the
releases on GitHub -- checked on every launch and on Refresh, cached under
~/.cache/armada2-refit/installer so it works offline -- and installs the chosen one by
running the release's own install.sh, following its progress. It can also write the
launch settings into Heroic's config for the game. Textures and the cutscene player are
not installed from here, for now.

It reads release packages of the schemas in SCHEMAS. The schema is the layout of the
release zip as this program relies on it (manifest.json, install.sh's ::step lines),
versioned on its own: gui-installer/README.md, "The package schema". Standalone: Python 3
and the standard library, plus PyGObject with GTK 4 and libadwaita for the window (the
AppImage carries them).
"""
import fnmatch
import glob
import hashlib
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
import zipfile

INSTALLER_VERSION = '1.1.0'
SCHEMAS = (0, 1)          # 0: the zips before manifest.json; 1: manifest.json, ::step lines
REPO = 'cedrichaase/armada2-refit'
API = f'https://api.github.com/repos/{REPO}/releases?per_page=30'
RELEASES_PAGE = f'https://github.com/{REPO}/releases'
INSTALLER_ASSET = 'armada2-refit-installer.py'
STATE_FILE = 'armada2-refit-installed.json'     # in the game directory: what we installed
GOG_APP = '1174788223'                          # Star Trek: Armada II on GOG
# Layers a package carries that this installer leaves out (install.sh's A2_SKIP): the
# cutscene player is not installed from here yet. Textures are not in a package at all.
EXCLUDED = ('cutscenes',)

HOME = os.path.expanduser('~')
XDG_CACHE = os.environ.get('XDG_CACHE_HOME') or os.path.join(HOME, '.cache')
XDG_CONFIG = os.environ.get('XDG_CONFIG_HOME') or os.path.join(HOME, '.config')
XDG_DATA = os.environ.get('XDG_DATA_HOME') or os.path.join(HOME, '.local/share')
CACHE = os.path.join(XDG_CACHE, 'armada2-refit', 'installer')
SETTINGS = os.path.join(XDG_CONFIG, 'armada2-refit', 'installer.json')

# The install.sh stages, in order, and what the window says during each.
STEPS = ['verify', 'prereqs', 'hud', 'menus', 'qol', 'lighting', 'online', 'msaa', 'cutscenes',
         'renderer', 'bloom', 'done']
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
    'online': 'Installing online play',
    'msaa': 'Installing anti-aliasing',
    'cutscenes': 'Skipping the cutscene player',
    'renderer': 'Configuring the renderer',
    'bloom': 'Setting up bloom',
    'done': 'Finishing up',
    'launcher': 'Writing Heroic launch settings',
    'uninstall': 'Removing the mod',
    'restore': 'Restoring Heroic launch settings',
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
    return '~' + path[len(HOME):] if path.startswith(HOME + '/') else path


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
    """(comm, cmdline) of every process we can see."""
    out = []
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
    def __init__(self, path, source, heroic=None):
        self.path, self.source, self.heroic = path, source, heroic

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
    base = root.rstrip('/').count('/')
    for d, subs, _ in os.walk(root):
        level = d.count('/') - base
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

    def add(path, source, heroic=None):
        if not path or not exe_in(path):
            return
        real = os.path.realpath(path)
        for g in found:
            if os.path.realpath(g.path) == real:
                g.heroic = g.heroic or heroic
                return
        found.append(Game(real, source, heroic))

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


# ------------------------------------------------------------------ the game's state

def is_dxvk(path):
    try:
        with open(path, 'rb') as f:
            return b'dxvk' in f.read().lower()
    except OSError:
        return False


D3D8TO9_SHA = '122928cfe225c25d30decf7184a5d37e490cecf3b58256ba3206c7e1853f8ab8'


def inspect(game):
    """What install.sh will find in the game directory -- the same tests it makes."""
    p = lambda n: os.path.join(game, n)
    d3d8to9 = os.path.isfile(p('d3d8.dll')) and sha256(p('d3d8.dll')) == D3D8TO9_SHA
    dxvk = is_dxvk(p('d3d8.dll')) or (d3d8to9 and is_dxvk(p('d3d9.dll')))
    layers = [os.path.join(XDG_DATA, 'vulkan/implicit_layer.d'), '/etc/vulkan/implicit_layer.d',
              '/usr/share/vulkan/implicit_layer.d']
    vkbasalt = any(glob.glob(os.path.join(d, '*[Bb]asalt*.json')) for d in layers)
    conf = p('dxvk.conf')
    foreign_conf = os.path.isfile(conf) and \
        'generated by tools/renderer-config.sh' not in open(conf, errors='replace').read()
    state = load_json(p(STATE_FILE))
    installed = state.get('version') if isinstance(state, dict) else None
    if not installed and os.path.exists(p('HUD.asi')):
        installed = '?'
    return dict(dxvk=dxvk, d3d8to9=d3d8to9, vkbasalt=vkbasalt, foreign_conf=foreign_conf,
                prereqs=os.path.exists(p('STA2WidescreenPatch.asi')),
                d3d9=os.path.exists(p('d3d9.dll')), installed=installed,
                state=state if isinstance(state, dict) else None)


def layer_verdict(layer, facts):
    """(will it go in, why) for one manifest layer on this game."""
    when = layer.get('when', 'always')
    if layer['id'] in EXCLUDED:
        return None, 'not available yet'
    if when == 'missing' and facts['prereqs']:
        return True, 'already there'
    if when == 'dxvk' and not facts['dxvk']:
        return False, 'skipped — needs DXVK in the game folder'
    if when == 'dxvk.conf' and facts['foreign_conf']:
        return False, 'skipped — your own dxvk.conf stays'
    if when == 'vkbasalt' and not facts['vkbasalt']:
        return False, 'skipped — no vkBasalt layer'
    if layer['id'] == 'lighting' and not facts['d3d8to9']:
        return True, 'will install · per vertex'
    return True, 'will install'


def launch_env(facts, package=None):
    """The launcher variables install.sh will print for this game."""
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


def http_get(url, timeout=15):
    req = urllib.request.Request(url, headers={
        'User-Agent': f'armada2-refit-installer/{INSTALLER_VERSION}',
        'Accept': 'application/vnd.github+json'})
    return urllib.request.urlopen(req, timeout=timeout)


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
        return self.schema in SCHEMAS

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

    def _run_script(self, pkgdir, args, steps, lo, hi):
        env = dict(os.environ, A2_PROGRESS='1', A2_SKIP=' '.join(EXCLUDED))
        cmd = ['bash', os.path.join(pkgdir, 'install.sh')] + args
        self.log('$ ' + ' '.join(cmd))
        proc = subprocess.Popen(cmd, cwd=pkgdir, env=env, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, text=True, bufsize=1)
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
            tail = [l for l in out if l.strip()][-1:] or ['install.sh failed']
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
            raise RuntimeError(f'release {pkg.version} uses package schema {pkg.schema}; '
                               f'this installer reads {", ".join(map(str, SCHEMAS))} — '
                               'download the newest installer')
        self.report(0.32, 'unpack', None)
        pkgdir = pkg.unpack(os.path.join(CACHE, 'unpacked', pkg.version))
        out = self._run_script(pkgdir, [game], pkg.steps, 0.35, 0.92)
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
                     heroic=old.get('heroic'))
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
        for n in notes:
            self.log('note: ' + n)
        save_json(os.path.join(game, STATE_FILE), state)
        self.report(1.0, 'done', None)
        return dict(version=pkg.version, env=wanted, notes=notes,
                    heroic=bool(state.get('heroic')))

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
        self._run_script(pkgdir, ['--uninstall', game], ['uninstall'], 0.1, 0.85)
        h = state.get('heroic')
        if h and os.path.exists(h.get('config', '')):
            self.report(0.9, 'restore', None)
            her = Heroic(os.path.dirname(os.path.dirname(h['config'])), h['app'], False)
            her.config = h['config']
            for c in heroic_apply(her, h.get('changes', []), undo=True):
                self.log(f'Heroic: {c["key"]} back to {c["old"] if c["old"] is not None else "(unset)"}')
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
        check(split_overrides('a=n,b; b=n') == {'a': 'n,b', 'b': 'n'}, 'overrides parse')
        check(vtuple('11.10.0') > vtuple('11.9.3'), 'versions sort as numbers')
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

CSS = """
window.a2 { background: #05070d; }
.sky-card { background: alpha(#10141f, 0.74); border: 1px solid alpha(white, 0.08);
            border-radius: 18px; }
.sky-card list, .sky-card row { background: transparent; }
.sky-card row { border-radius: 0; }
.sky-card > list > row:first-child { border-radius: 18px 18px 0 0; }
.sky-card > list > row:last-child { border-radius: 0 0 18px 18px; }
.title-big { font-size: 30px; font-weight: 300; letter-spacing: 0.5px; }
.title-sub { color: alpha(@window_fg_color, 0.6); }
.status { color: alpha(@window_fg_color, 0.75); }
.status.err { color: @error_color; }
.status.ok { color: @success_color; }
.heads-up { color: alpha(@window_fg_color, 0.7); font-size: 0.92em; }
.heads-up.warn { color: @warning_color; }
progressbar.thin trough, progressbar.thin progress { min-height: 4px; }
progressbar.thin trough { background: alpha(white, 0.10); }
.mono { font-family: monospace; font-size: 0.9em; }
.logview { background: alpha(black, 0.35); padding: 8px; }
"""


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
        """The window's background: a dark gradient, a faint nebula and three layers of
        stars drifting at different speeds. Drawn in code, so there is nothing to ship;
        still when the desktop has animations off."""
        LAYERS = ((90, 0.55, 3.0), (60, 0.85, 7.0), (28, 1.35, 14.0))   # count, radius, px/s

        def __init__(self):
            super().__init__(hexpand=True, vexpand=True)
            rnd = random.Random(2399)
            self.stars = [[rnd.random(), rnd.random(), r, rnd.uniform(0.35, 1.0),
                           rnd.uniform(0, 6.3), speed, rnd.choice((0, 0, 0, 1, 2))]
                          for count, r, speed in self.LAYERS for _ in range(count)]
            self.tints = ((1.0, 1.0, 1.0), (0.75, 0.85, 1.0), (1.0, 0.88, 0.75))
            self.t0 = None
            self.nebula = None
            self.animate = Gtk.Settings.get_default().get_property('gtk-enable-animations')
            self.set_draw_func(self.draw)
            if self.animate:
                self.add_tick_callback(self.tick)
            self._last = 0

        def tick(self, widget, clock):
            now = clock.get_frame_time()
            if now - self._last >= 33000:       # about 30 frames a second is plenty
                self._last = now
                self.queue_draw()
            return GLib.SOURCE_CONTINUE

        def build_nebula(self, w, h):
            import cairo
            s = cairo.ImageSurface(cairo.FORMAT_ARGB32, w, h)
            cr = cairo.Context(s)
            g = cairo.LinearGradient(0, 0, 0, h)
            g.add_color_stop_rgb(0, 0.016, 0.022, 0.050)
            g.add_color_stop_rgb(1, 0.030, 0.040, 0.085)
            cr.set_source(g)
            cr.paint()
            for fx, fy, fr, col in ((0.18, 0.22, 0.75, (0.22, 0.30, 0.75, 0.16)),
                                    (0.85, 0.70, 0.80, (0.45, 0.20, 0.65, 0.12)),
                                    (0.55, 0.05, 0.55, (0.15, 0.45, 0.65, 0.07))):
                rad = cairo.RadialGradient(fx * w, fy * h, 0, fx * w, fy * h, fr * max(w, h))
                rad.add_color_stop_rgba(0, *col)
                rad.add_color_stop_rgba(1, col[0], col[1], col[2], 0)
                cr.set_source(rad)
                cr.paint()
            self.nebula = (w, h, s)

        def draw(self, area, cr, w, h):
            if not self.nebula or self.nebula[:2] != (w, h):
                self.build_nebula(w, h)
            cr.set_source_surface(self.nebula[2], 0, 0)
            cr.paint()
            now = (GLib.get_monotonic_time() / 1e6) if self.animate else 0.0
            for x, y, r, a, ph, speed, tint in self.stars:
                px = (x * w + now * speed) % w
                tw = 0.75 + 0.25 * math.sin(now * 0.9 + ph) if self.animate else 1.0
                cr.set_source_rgba(*self.tints[tint], a * tw)
                cr.arc(px, y * h, r, 0, 6.2832)
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
            overlay.set_child(Sky())

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
            title = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
            title.append(lab('Armada II Refit', 'title-big', xalign=0.5))
            title.append(lab('Remastering tools for Star Trek: Armada II', 'title-sub', xalign=0.5))
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

            self.row_heroic = Adw.SwitchRow(title='Heroic launch settings', active=True,
                                            subtitle_lines=3)
            self.row_heroic.connect('notify::active', lambda *_: self.update_launcher())
            rows.append(self.row_heroic)

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
            self.b_install = Gtk.Button(label='Install', halign=Gtk.Align.CENTER)
            self.b_install.add_css_class('suggested-action')
            self.b_install.add_css_class('pill')
            self.b_install.set_size_request(220, 48)
            self.b_install.connect('clicked', lambda _b: self.on_install())
            act.append(self.b_install)
            self.b_uninstall = Gtk.Button(label='Uninstall', halign=Gtk.Align.CENTER)
            self.b_uninstall.add_css_class('flat')
            self.b_uninstall.connect('clicked', lambda _b: self.on_uninstall())
            act.append(self.b_uninstall)
            col.append(act)

            prog = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
            self.bar = Gtk.ProgressBar()
            self.bar.add_css_class('thin')
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
            for w in (self.dd_game, self.dd_rel, self.row_heroic):
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
            notes.append('Textures are built from your own files, not shipped in releases.')
            for i, n in enumerate(notes):
                self.heads_up.append(lab(n, ('heads-up', 'warn') if i < len(notes) - 1
                                         else 'heads-up', xalign=0.5, wrap=True,
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
            self.row_env.set_visible(bool(env) and not (heroic and self.row_heroic.get_active()))
            if not heroic:
                return
            changes, conflicts = heroic_plan(g.heroic, env)
            if changes is None:
                sub = conflicts[0]
            elif not changes:
                sub = f'Already set in {g.heroic.label()}'
            else:
                sub = f'Adds {", ".join(c["key"] for c in changes)} to {g.heroic.label()}'
            if conflicts and changes is not None:
                sub += '. ' + '; '.join(conflicts)
            if self.row_heroic.get_active() and heroic_running():
                sub += '. Heroic is running: restart it afterwards'
            self.row_heroic.set_subtitle(GLib.markup_escape_text(sub))

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
            label = 'Install'
            if r and inst:
                label = ('Reinstall' if inst == r.version else 'Install' if inst == '?' else
                         'Upgrade to' if vtuple(r.version) > vtuple(inst) else 'Switch to')
                if label != 'Reinstall' and label != 'Install':
                    label += f' {r.version}'
            self.b_install.set_label(label)

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
            write = self.row_heroic.get_active() and bool(g.heroic)
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
            self.bar.set_fraction(0)
            self.log(f'== install {r.version} into {g.path}')

            def done(res):
                self.set_busy(False)
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
    if hasattr(provider, 'load_from_string'):
        provider.load_from_string(CSS)
    else:
        provider.load_from_data(CSS.encode(), -1)

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


if __name__ == '__main__':
    if len(sys.argv) > 1:
        sys.exit(cli(sys.argv[1:]))
    try:
        sys.exit(run_gui())
    except (ImportError, ValueError) as e:
        print(f'The window needs PyGObject with GTK 4 and libadwaita ({e}).\n'
              'Use the AppImage, which carries them, or the command line: --help.',
              file=sys.stderr)
        sys.exit(1)
