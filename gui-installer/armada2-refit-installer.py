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
~/.cache/armada2-refit/installer so it works offline -- shows what the chosen release
installs, and installs it by running the release's own install.sh, following its
progress. It can also write the launch settings into Heroic's config for the game.
Textures and the cutscene player are not installed from here, for now.

It reads release packages of the schemas in SCHEMAS. The schema is the layout of the
release zip as this program relies on it (manifest.json, install.sh's ::step lines),
versioned on its own: gui-installer/README.md, "The package schema". Standalone: Python 3
and the standard library, plus PyGObject with GTK 4 and libadwaita for the window.
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

INSTALLER_VERSION = '1.0.0'
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
# Antonio (SIL OFL 1.1, Vernon Adams), the condensed face of the interface; fetched once
# from Google Fonts' repository, pinned by commit and hash. Without it: a fallback face.
FONT_URL = ('https://raw.githubusercontent.com/google/fonts/'
            '95f4904fc8bcf26d3420fe315560c96417c6dec7/ofl/antonio/Antonio%5Bwght%5D.ttf')
FONT_SHA = '9e95a2258ecdf3e45c72c5bbea1c4cd350e8f7bebc87c9dba53b29b1890b8903'

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
    'fetch': 'Contacting Starfleet archives',
    'download': 'Downloading release {version}',
    'unpack': 'Unpacking the package',
    'verify': 'Verifying package integrity',
    'prereqs': 'Fitting the widescreen patch and ASI loader',
    'hud': 'Refitting the HUD',
    'menus': 'Widening the menus',
    'qol': 'Loading quality-of-life subroutines',
    'lighting': 'Installing lighting',
    'online': 'Opening subspace channels for online play',
    'msaa': 'Smoothing the edges',
    'cutscenes': 'Leaving the cutscenes stock',
    'renderer': 'Tuning the renderer',
    'bloom': 'Charging the bloom emitters',
    'done': 'Finishing up',
    'launcher': 'Writing Heroic launch settings',
    'uninstall': 'Taking the mod out',
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
NOT_INCLUDED = [dict(id='textures', name='Textures',
                     summary='remastered textures are built from your own game files with '
                             './a2tex; a release cannot carry them')]


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
        self.notes = kw.get('notes', '')
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
                    notes=self.notes, url=self.url, size=self.size, digest=self.digest,
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
                           date=(rel.get('published_at') or '')[:10], notes=rel.get('body') or '',
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
            notes = ''
            try:
                notes = Package(z).changelog()
            except Exception:
                pass
            rels.append(Release(m.group(1), notes=notes, size=os.path.getsize(z)))
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
        self.not_included = m.get('not_included', NOT_INCLUDED)
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

    def changelog(self):
        return f'Release {self.version}, from the cache.'

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


# ------------------------------------------------------------------ settings, font

def settings():
    return load_json(SETTINGS, {}) or {}


def remember(**kw):
    s = settings()
    s.update(kw)
    save_json(SETTINGS, s)


def font_path(fetch=False):
    p = os.path.join(CACHE, 'fonts', 'Antonio.ttf')
    if os.path.isfile(p) and sha256(p) == FONT_SHA:
        return p
    if not fetch:
        return None
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with http_get(FONT_URL) as r:
        data = r.read()
    if hashlib.sha256(data).hexdigest() != FONT_SHA:
        return None
    with open(p, 'wb') as f:
        f.write(data)
    return p


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
    a = ap.parse_args(argv)
    if a.check_version is not None:
        print(INSTALLER_VERSION)
        return 0 if a.check_version == INSTALLER_VERSION else 1
    if a.selftest:
        return selftest()

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
    return 0 if ok else 1


# ------------------------------------------------------------------ the window

CSS = """
@define-color lc_orange #ff9900;
@define-color lc_peach #ffcc99;
@define-color lc_lilac #cc99cc;
@define-color lc_blue #9999ff;
@define-color lc_sky #99ccff;
@define-color lc_red #dd5555;
@define-color lc_gold #ffaa00;
@define-color lc_tan #d6a77a;
@define-color lc_dim #6b5a4a;

window, .lcars-root { background: #000; color: @lc_peach; }
.lcars, .lcars label { font-family: "Antonio", "Oswald", "Bebas Neue", "League Gothic",
    "Liberation Sans Narrow", "DejaVu Sans Condensed", sans-serif; }
.title { font-size: 40px; font-weight: 600; color: @lc_orange; letter-spacing: 2px; }
.subtitle { font-size: 15px; color: #000; font-weight: 600; letter-spacing: 1px; }
.bar { min-height: 10px; }
.c-orange { background: @lc_orange; } .c-peach { background: @lc_peach; }
.c-lilac { background: @lc_lilac; } .c-blue { background: @lc_blue; }
.c-sky { background: @lc_sky; } .c-red { background: @lc_red; }
.c-gold { background: @lc_gold; } .c-tan { background: @lc_tan; }
.cap-r { border-radius: 0 999px 999px 0; }
.cap-l { border-radius: 999px 0 0 999px; }

button.side { border-radius: 0; border: none; box-shadow: none; background-image: none;
    min-height: 58px; padding: 0 10px 4px 0; margin: 0; }
button.side label { color: #000; font-size: 18px; font-weight: 600; letter-spacing: 1px; }
button.side:hover { filter: brightness(1.15); }
button.side:checked { background: @lc_peach; }
.side-fill { background: @lc_lilac; }
.side-code { color: #000; font-size: 13px; font-weight: 600; }

button.pill { border-radius: 999px; border: none; box-shadow: none; background-image: none;
    min-height: 40px; padding: 0 26px; }
button.pill label { color: #000; font-size: 19px; font-weight: 600; letter-spacing: 1px; }
button.pill:hover { filter: brightness(1.15); }
button.pill:disabled { background: #3a3530; } button.pill:disabled label { color: #777; }
button.pill.small { min-height: 30px; padding: 0 16px; }
button.pill.small label { font-size: 15px; }

.section { color: @lc_orange; font-size: 22px; font-weight: 600; letter-spacing: 2px; }
.section-bar { min-width: 18px; min-height: 22px; border-radius: 999px 0 0 999px; }
.key { color: @lc_lilac; font-size: 16px; letter-spacing: 1px; }
.val { color: @lc_peach; font-size: 16px; }
.val.good { color: @lc_sky; } .val.warn { color: @lc_gold; } .val.bad { color: @lc_red; }
.lcars label.prose, .prose { color: #e8d8c8; font-family: sans-serif; font-size: 13px; }
.lcars label.dimtext, .dimtext { color: #a08c78; font-family: sans-serif; font-size: 12px; }
.panel { border: 2px solid #2b2420; border-radius: 18px; padding: 14px 16px; background: #080605; }

.chip { border-radius: 999px; padding: 2px 14px; min-width: 150px; }
.chip label { color: #000; font-size: 16px; font-weight: 600; letter-spacing: 1px; }
.chip.off { background: #3a3530; } .chip.off label { color: #8a8076; }
.lver { color: @lc_lilac; font-size: 16px; min-width: 56px; }
.verdict { font-size: 15px; letter-spacing: 1px; }
.verdict.yes { color: @lc_sky; } .verdict.no { color: @lc_gold; } .verdict.na { color: #8a8076; }
.layer-row { padding: 6px 0; border-bottom: 1px solid #1c1714; }

.status { color: @lc_orange; font-size: 22px; letter-spacing: 2px; font-weight: 600; }
.status.err { color: @lc_red; } .status.ok { color: @lc_sky; }
.banner { background: #1d1408; border-radius: 12px; padding: 8px 14px; }
.banner label { color: @lc_gold; }

dropdown > button { background: #15110e; border-radius: 999px; border: 2px solid @lc_dim;
    box-shadow: none; background-image: none; min-height: 34px; }
dropdown > button label { color: @lc_peach; font-family: "Antonio", sans-serif; font-size: 16px; }
switch { background: #3a3530; } switch:checked { background: @lc_orange; }
switch > slider { background: #000; }
textview, textview text { background: #050403; color: #e8d8c8; }
textview.mono text { font-family: monospace; font-size: 12px; color: @lc_peach; }
scrollbar slider { background: @lc_dim; }
"""


def run_gui():
    import gi
    gi.require_version('Gtk', '4.0')
    gi.require_version('Adw', '1')
    gi.require_version('PangoCairo', '1.0')
    from gi.repository import Adw, Gdk, Gio, GLib, Gtk, PangoCairo
    import math

    def add_font(path):
        try:
            return PangoCairo.FontMap.get_default().add_font_file(path)
        except Exception:
            return False

    fp = font_path()
    if fp:
        add_font(fp)

    def lab(text='', cls=(), xalign=0.0, wrap=False, **kw):
        w = Gtk.Label(label=text, xalign=xalign, **kw)
        for c in cls:
            w.add_css_class(c)
        if wrap:
            w.set_wrap(True)
            w.set_natural_wrap_mode(Gtk.NaturalWrapMode.WORD)
        return w

    def seg(color, width=-1, height=-1, hexpand=False, extra=()):
        b = Gtk.Box()
        b.add_css_class('bar')
        b.add_css_class('c-' + color)
        for c in extra:
            b.add_css_class(c)
        b.set_size_request(width, height)
        b.set_hexpand(hexpand)
        return b

    def pill(text, color, small=False):
        b = Gtk.Button()
        b.set_child(lab(text.upper(), xalign=0.5))
        for c in ('pill', 'c-' + color, 'lcars') + (('small',) if small else ()):
            b.add_css_class(c)
        return b

    SIDE_W, RI, R = 168, 26, 54
    RGB = {'orange': (1, .6, 0), 'lilac': (.8, .6, .8), 'blue': (.6, .6, 1),
           'peach': (1, .8, .6), 'tan': (.84, .65, .48)}

    class Elbow(Gtk.DrawingArea):
        def __init__(self, color, bar_h, top):
            super().__init__()
            self.color, self.bar_h, self.top = RGB[color], bar_h, top
            self.set_content_width(SIDE_W + RI + 12)
            self.set_content_height(bar_h + RI + 10)
            self.set_draw_func(self.draw)

        def draw(self, area, cr, w, h):
            if not self.top:
                cr.translate(0, h)
                cr.scale(1, -1)
            H = self.bar_h
            cr.move_to(0, h)
            cr.line_to(0, R)
            cr.arc(R, R, R, math.pi, 1.5 * math.pi)
            cr.line_to(w, 0)
            cr.line_to(w, H)
            cr.line_to(SIDE_W + RI, H)
            cr.arc_negative(SIDE_W + RI, H + RI, RI, 1.5 * math.pi, math.pi)
            cr.line_to(SIDE_W, h)
            cr.close_path()
            cr.set_source_rgb(*self.color)
            cr.fill()

    class Segments(Gtk.DrawingArea):
        """The progress bar: a row of LCARS blocks that fill, with a running light."""
        N = 36

        def __init__(self):
            super().__init__()
            self.fraction, self.busy, self.err, self.phase = 0.0, False, False, 0
            self.set_content_height(26)
            self.set_hexpand(True)
            self.set_draw_func(self.draw)
            GLib.timeout_add(70, self.tick)

        def tick(self):
            if self.busy:
                self.phase = (self.phase + 1) % (self.N * 2)
                self.queue_draw()
            return True

        def set(self, fraction, busy=None, err=False):
            self.fraction = max(0.0, min(1.0, fraction))
            if busy is not None:
                self.busy = busy
            self.err = err
            self.queue_draw()

        def draw(self, area, cr, w, h):
            gap = 4
            bw = (w - gap * (self.N - 1)) / self.N
            full = self.fraction * self.N
            for i in range(self.N):
                x = i * (bw + gap)
                r = min(h / 2, 8) if i in (0, self.N - 1) else 2
                self.rect(cr, x, 0, bw, h, r, left=(i == 0), right=(i == self.N - 1))
                if self.err and i < max(full, 1):
                    cr.set_source_rgb(.87, .33, .33)
                elif i < int(full):
                    t = i / self.N
                    cr.set_source_rgb(1, .6 + .2 * t, .0 + .6 * t)
                elif i == int(full) and self.busy:
                    k = .5 + .5 * math.sin(self.phase / 2.0)
                    cr.set_source_rgb(.4 + .6 * k, .3 + .3 * k, .1)
                else:
                    cr.set_source_rgb(.16, .13, .11)
                cr.fill()
            if self.busy:   # a sweep, as on a sensor display
                x = (self.phase / (self.N * 2)) * w
                cr.set_source_rgba(.6, .8, 1, .18)
                cr.rectangle(x - 30, 0, 60, h)
                cr.fill()

        @staticmethod
        def rect(cr, x, y, w, h, r, left, right):
            rl = r if left else 2
            rr = r if right else 2
            cr.new_sub_path()
            cr.arc(x + w - rr, y + rr, rr, -math.pi / 2, 0)
            cr.arc(x + w - rr, y + h - rr, rr, 0, math.pi / 2)
            cr.arc(x + rl, y + h - rl, rl, math.pi / 2, math.pi)
            cr.arc(x + rl, y + rl, rl, math.pi, 1.5 * math.pi)
            cr.close_path()

    class Window(Adw.ApplicationWindow):
        def __init__(self, app):
            super().__init__(application=app, title='Armada II Refit — Installer')
            self.set_default_size(1280, 820)
            self.games, self.releases, self.package = [], [], None
            self.busy, self.checked, self.check_error = False, None, None
            self.updating = False
            self.build()
            self.load_games()
            rels, self.checked = cached_releases()
            self.set_releases(rels, keep=False)
            self.refresh()
            if not fp:
                threading.Thread(target=self.fetch_font, daemon=True).start()

        # ---------------------------------------------------------- layout
        def build(self):
            grid = Gtk.Grid()
            grid.add_css_class('lcars-root')
            grid.add_css_class('lcars')
            grid.set_margin_top(14)
            grid.set_margin_bottom(14)
            grid.set_margin_start(14)
            grid.set_margin_end(14)
            self.set_content(grid)

            # Top: the elbow, and the title bar.
            grid.attach(Elbow('orange', 46, True), 0, 0, 1, 1)
            top = Gtk.Box(spacing=6, valign=Gtk.Align.START)
            top.set_size_request(-1, 46)
            top.append(seg('orange', hexpand=True))
            title = lab('ARMADA II REFIT', ('title',))
            title.set_margin_start(10)
            title.set_margin_end(10)
            title.set_valign(Gtk.Align.CENTER)
            top.append(title)
            tag = Gtk.Box()
            tag.add_css_class('c-lilac')
            tag.set_size_request(150, -1)
            tl = lab(f'INSTALLER {INSTALLER_VERSION}', ('subtitle',), xalign=1)
            tl.set_hexpand(True)
            tl.set_margin_end(10)
            tl.set_valign(Gtk.Align.END)
            tag.append(tl)
            top.append(tag)
            top.append(seg('blue', width=46, extra=('cap-r',)))
            grid.attach(top, 1, 0, 1, 1)

            # Left: the navigation.
            side = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=5,
                           halign=Gtk.Align.START)
            side.set_size_request(SIDE_W, -1)
            side.set_margin_top(5)
            side.set_margin_bottom(5)
            self.stack = Gtk.Stack(transition_type=Gtk.StackTransitionType.CROSSFADE)
            self.nav = {}
            first = None
            for i, (name, color) in enumerate([('overview', 'orange'), ('notes', 'blue'),
                                               ('launcher', 'tan'), ('log', 'lilac')]):
                b = Gtk.ToggleButton()
                lb = lab(f'{i + 1:02d}-{name.upper()}', xalign=1, yalign=1)
                lb.set_valign(Gtk.Align.END)
                b.set_child(lb)
                b.add_css_class('side')
                b.add_css_class('c-' + color)
                if first:
                    b.set_group(first)
                else:
                    first = b
                b.connect('toggled', self.on_nav, name)
                side.append(b)
                self.nav[name] = b
            fill = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, vexpand=True)
            fill.add_css_class('side-fill')
            code = lab(f'GOG {GOG_APP}', ('side-code',), xalign=1)
            code.set_valign(Gtk.Align.END)
            code.set_vexpand(True)
            code.set_margin_end(10)
            code.set_margin_bottom(6)
            fill.append(code)
            side.append(fill)
            side.append(seg('orange', height=34))
            self.sensor = seg('red', height=18)
            side.append(self.sensor)
            grid.attach(side, 0, 1, 1, 1)

            # Bottom: the elbow and its bar.
            grid.attach(Elbow('tan', 22, False), 0, 2, 1, 1)
            bot = Gtk.Box(spacing=6, valign=Gtk.Align.END)
            bot.set_size_request(-1, 22)
            bot.append(seg('tan', width=220))
            bot.append(seg('lilac', width=60))
            self.footer = lab('', ('dimtext',), xalign=1)
            self.footer.set_hexpand(True)
            self.footer.set_margin_end(6)
            bot.append(self.footer)
            bot.append(seg('peach', width=90))
            bot.append(seg('blue', width=30, extra=('cap-r',)))
            grid.attach(bot, 1, 2, 1, 1)

            # The content, and below it the status line, progress and the buttons.
            main = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12,
                           hexpand=True, vexpand=True)
            main.set_margin_top(4)
            main.set_margin_bottom(8)
            main.set_margin_end(4)
            self.stack.set_vexpand(True)
            main.append(self.stack)
            self.stack.add_named(self.page_overview(), 'overview')
            self.stack.add_named(self.page_notes(), 'notes')
            self.stack.add_named(self.page_launcher(), 'launcher')
            self.stack.add_named(self.page_log(), 'log')

            self.status = lab('STANDING BY', ('status',))
            self.status.set_ellipsize(3)
            main.append(self.status)
            self.bar = Segments()
            main.append(self.bar)
            row = Gtk.Box(spacing=10)
            self.detail = lab('', ('dimtext',), wrap=True)
            self.detail.set_hexpand(True)
            row.append(self.detail)
            self.b_refresh = pill('Refresh', 'blue')
            self.b_refresh.connect('clicked', lambda *_: self.refresh())
            self.b_uninstall = pill('Uninstall', 'red')
            self.b_uninstall.connect('clicked', lambda *_: self.on_uninstall())
            self.b_install = pill('Engage', 'orange')
            self.b_install.connect('clicked', lambda *_: self.on_install())
            for b in (self.b_refresh, self.b_uninstall, self.b_install):
                row.append(b)
            main.append(row)
            grid.attach(main, 1, 1, 1, 1)
            first.set_active(True)

            # F5 refreshes, Ctrl+Enter installs, Alt+1..4 switch pages.
            keys = Gtk.ShortcutController()
            keys.set_scope(Gtk.ShortcutScope.GLOBAL)

            def key(trigger, fn):
                keys.add_shortcut(Gtk.Shortcut.new(Gtk.ShortcutTrigger.parse_string(trigger),
                                                   Gtk.CallbackAction.new(lambda *_: fn() or True)))
            key('F5', self.refresh)
            key('<Control>Return', lambda: self.b_install.get_sensitive() and self.on_install())
            for i, name in enumerate(('overview', 'notes', 'launcher', 'log'), 1):
                key(f'<Alt>{i}', lambda n=name: self.nav[n].set_active(True))
            self.add_controller(keys)

        def section(self, text, color='orange'):
            b = Gtk.Box(spacing=10)
            bar = seg(color, extra=('section-bar',))
            bar.set_valign(Gtk.Align.CENTER)
            b.append(bar)
            b.append(lab(text.upper(), ('section',)))
            return b

        def facts_grid(self):
            g = Gtk.Grid(column_spacing=18, row_spacing=4)
            g.rows = {}
            return g

        def fact(self, g, key, value, kind=''):
            if key not in g.rows:
                k = lab(key.upper(), ('key',))
                v = lab('', ('val',), wrap=True)
                v.set_hexpand(True)
                v.set_selectable(True)
                n = len(g.rows)
                g.attach(k, 0, n, 1, 1)
                g.attach(v, 1, n, 1, 1)
                g.rows[key] = v
            v = g.rows[key]
            v.set_label(value)
            for c in ('good', 'warn', 'bad'):
                v.remove_css_class(c)
            if kind:
                v.add_css_class(kind)

        def page_overview(self):
            box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
            box.set_margin_end(12)
            self.banner = Gtk.Box(spacing=12)
            self.banner.add_css_class('banner')
            self.banner_text = lab('', wrap=True)
            self.banner_text.set_hexpand(True)
            self.banner.append(self.banner_text)
            self.banner_link = pill('Download', 'gold', small=True)
            self.banner_link.connect('clicked', lambda *_: self.open_uri(self.banner_uri))
            self.banner.append(self.banner_link)
            self.banner.set_visible(False)
            box.append(self.banner)

            cols = Gtk.Box(spacing=14, homogeneous=True)
            # The game.
            p = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
            p.add_css_class('panel')
            p.append(self.section('Game'))
            row = Gtk.Box(spacing=8)
            self.dd_game = Gtk.DropDown.new_from_strings(['searching…'])
            self.dd_game.set_hexpand(True)
            self.dd_game.connect('notify::selected', lambda *_: self.on_game())
            row.append(self.dd_game)
            browse = pill('Browse', 'lilac', small=True)
            browse.connect('clicked', lambda *_: self.on_browse())
            row.append(browse)
            p.append(row)
            self.g_game = self.facts_grid()
            p.append(self.g_game)
            cols.append(p)
            # The release.
            p = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
            p.add_css_class('panel')
            p.append(self.section('Release', 'blue'))
            self.dd_rel = Gtk.DropDown.new_from_strings(['no releases yet'])
            self.dd_rel.set_hexpand(True)
            self.dd_rel.connect('notify::selected', lambda *_: self.on_release())
            p.append(self.dd_rel)
            self.g_rel = self.facts_grid()
            p.append(self.g_rel)
            cols.append(p)
            box.append(cols)

            box.append(self.section('What it installs', 'lilac'))
            self.layers_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
            box.append(self.layers_box)
            sw = Gtk.ScrolledWindow(hscrollbar_policy=Gtk.PolicyType.NEVER)
            sw.set_child(box)
            return sw

        def page_notes(self):
            box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
            self.notes_title = self.section('Release notes', 'blue')
            box.append(self.notes_title)
            self.notes = Gtk.TextView(editable=False, cursor_visible=False,
                                      wrap_mode=Gtk.WrapMode.WORD)
            self.notes.set_left_margin(14)
            self.notes.set_right_margin(14)
            self.notes.set_top_margin(10)
            buf = self.notes.get_buffer()
            buf.create_tag('h', foreground='#ff9900', scale=1.35, weight=700,
                           family='Antonio', pixels_above_lines=10)
            buf.create_tag('h3', foreground='#cc99cc', scale=1.15, weight=700,
                           family='Antonio', pixels_above_lines=8)
            buf.create_tag('code', foreground='#99ccff', family='monospace')
            buf.create_tag('bullet', foreground='#ff9900')
            sw = Gtk.ScrolledWindow(vexpand=True)
            sw.set_child(self.notes)
            box.append(sw)
            return box

        def page_launcher(self):
            box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
            box.set_margin_end(12)
            box.append(self.section('Launch settings', 'tan'))
            box.append(lab('Wine loads its own DLLs unless told otherwise, so the game needs '
                           'these variables in its launcher. The installer can write them into '
                           "Heroic's settings for Armada II (a DLL override already there is "
                           'kept and the missing ones added; a variable already set to something '
                           'else is left alone). Uninstalling takes out what it added.',
                           ('prose',), wrap=True))
            row = Gtk.Box(spacing=12)
            self.sw_heroic = Gtk.Switch(valign=Gtk.Align.CENTER)
            self.sw_heroic.set_active(settings().get('heroic', True))
            self.sw_heroic.connect('notify::active', lambda *_: (
                remember(heroic=self.sw_heroic.get_active()), self.update_launcher()))
            row.append(self.sw_heroic)
            self.heroic_label = lab('', ('val',), wrap=True)
            self.heroic_label.set_hexpand(True)
            row.append(self.heroic_label)
            box.append(row)
            self.g_env = Gtk.Grid(column_spacing=18, row_spacing=6)
            box.append(self.g_env)
            box.append(self.section('For any other launcher', 'lilac'))
            self.env_text = Gtk.TextView(editable=False, monospace=True)
            self.env_text.add_css_class('mono')
            self.env_text.set_top_margin(8)
            self.env_text.set_left_margin(10)
            self.env_text.set_bottom_margin(8)
            box.append(self.env_text)
            cp = pill('Copy', 'lilac', small=True)
            cp.set_halign(Gtk.Align.START)
            cp.connect('clicked', lambda *_: self.copy_env())
            box.append(cp)
            sw = Gtk.ScrolledWindow(hscrollbar_policy=Gtk.PolicyType.NEVER)
            sw.set_child(box)
            return sw

        def page_log(self):
            box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
            box.append(self.section('Log', 'lilac'))
            self.logview = Gtk.TextView(editable=False, cursor_visible=False, monospace=True)
            self.logview.add_css_class('mono')
            self.logview.set_left_margin(10)
            self.logview.set_top_margin(8)
            sw = Gtk.ScrolledWindow(vexpand=True)
            sw.set_child(self.logview)
            self.logscroll = sw
            box.append(sw)
            return box

        # ---------------------------------------------------------- helpers
        def on_nav(self, button, name):
            if button.get_active():
                self.stack.set_visible_child_name(name)

        def open_uri(self, uri):
            Gtk.UriLauncher.new(uri).launch(self, None, None, None)

        def log(self, line):
            buf = self.logview.get_buffer()
            buf.insert(buf.get_end_iter(), line + '\n')
            adj = self.logscroll.get_vadjustment()
            GLib.idle_add(lambda: adj.set_value(adj.get_upper()))

        def set_status(self, text, kind=''):
            self.status.set_label(text.upper())
            for c in ('err', 'ok'):
                self.status.remove_css_class(c)
            if kind:
                self.status.add_css_class(kind)

        def set_busy(self, busy):
            self.busy = busy
            self.bar.set(self.bar.fraction, busy=busy)
            self.dd_game.set_sensitive(not busy)
            self.dd_rel.set_sensitive(not busy)
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
            self.bar.set(self.bar.fraction, busy=False, err=True)
            self.set_status(f'Failed: {e}', 'err')
            self.log(f'error: {e}')
            self.sensor.set_visible(True)

        # ---------------------------------------------------------- the game
        def load_games(self, select=None):
            self.games = find_games()
            last = select or settings().get('game')
            if last and exe_in(last) and not any(
                    os.path.realpath(g.path) == os.path.realpath(last) for g in self.games):
                self.games.insert(0, Game(os.path.realpath(last), 'Chosen'))
            self.updating = True
            labels = [g.label() for g in self.games] or ['no Armada II found — Browse']
            self.dd_game.set_model(Gtk.StringList.new(labels))
            idx = next((i for i, g in enumerate(self.games) if last and
                        os.path.realpath(g.path) == os.path.realpath(last)), 0)
            self.dd_game.set_selected(idx)
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
                    self.set_status('No Armada2.exe in that folder', 'err')
                    return
                remember(game=path)
                self.load_games(select=path)
            d.select_folder(self, None, done)

        def on_game(self):
            if self.updating:
                return
            g = self.game()
            gg = self.g_game
            if not g:
                self.fact(gg, 'Folder', 'not found — use Browse to point at the folder '
                                        'holding Armada2.exe', 'bad')
                self.facts = None
                self.update_all()
                return
            remember(game=g.path)
            f = self.facts = inspect(g.path)
            self.fact(gg, 'Folder', short(g.path))
            self.fact(gg, 'Found by', g.source)
            inst = f['installed']
            self.fact(gg, 'Installed', ('Refit ' + inst if inst != '?' else 'Refit, version unknown')
                      if inst else 'stock game', 'good' if inst else '')
            self.fact(gg, 'Direct3D', ('DXVK through d3d8to9 · per-pixel lighting' if f['d3d8to9'] and f['dxvk']
                                       else 'DXVK' if f['dxvk']
                                       else 'Proton’s own · no MSAA, lighting per vertex'),
                      'good' if f['dxvk'] else 'warn')
            self.fact(gg, 'Bloom', 'vkBasalt found' if f['vkbasalt']
                      else 'no vkBasalt layer — bloom skipped (optional)',
                      'good' if f['vkbasalt'] else 'warn')
            self.fact(gg, 'Launcher', f'{g.heroic.label()}, GOG app {g.heroic.app}' if g.heroic
                      else 'not Heroic — set the variables by hand', '' if g.heroic else 'warn')
            self.update_all()

        # ---------------------------------------------------------- releases
        def set_releases(self, rels, keep=True):
            cur = self.release()
            cur_v = cur.version if cur and keep and getattr(self, 'user_picked', False) else None
            self.releases = rels
            self.updating = True
            labels = []
            newest = default_release(rels)
            for r in rels:
                tags = [r.date] if r.date else []
                if r.prerelease:
                    tags.append('PRE-RELEASE')
                if r is newest:
                    tags.append('NEWEST')
                if r.is_cached():
                    tags.append('CACHED')
                if not r.url:
                    tags.append('NO LONGER ON GITHUB')
                labels.append(f'{r.version}  ·  ' + ' · '.join(tags))
            self.dd_rel.set_model(Gtk.StringList.new(labels or ['no releases yet — Refresh']))
            idx = 0
            pick = cur_v or (newest.version if newest else None)
            for i, r in enumerate(rels):
                if r.version == pick:
                    idx = i
            self.dd_rel.set_selected(idx)
            self.updating = False
            self.on_release(user=False)

        def refresh(self):
            if self.busy:
                return
            self.set_busy(True)
            self.set_status(STATUS['fetch'] + '…')
            self.bar.set(0.0, busy=True)
            self.load_games(select=self.game().path if self.game() else None)

            def done(rels):
                self.checked, self.check_error = time.time(), None
                self.set_busy(False)
                self.bar.set(0.0, busy=False)
                newest = default_release(rels)
                self.set_status(f'{len(rels)} releases on record · newest {newest.version}'
                                if newest else 'No releases found', 'ok' if newest else 'err')
                self.set_releases(rels)

            def fail(e):
                self.check_error = str(e)
                self.set_busy(False)
                self.bar.set(0.0, busy=False)
                self.set_status('Offline — showing cached releases', 'err')
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
            g = self.g_rel
            if not r:
                self.fact(g, 'Status', 'nothing to install yet — check your connection, '
                                       'then Refresh', 'bad')
                self.update_all()
                return
            self.fact(g, 'Version', r.version)
            self.fact(g, 'Channel', 'pre-release — not yet seen in game' if r.prerelease
                      else 'release', 'warn' if r.prerelease else 'good')
            self.fact(g, 'Published', r.date or 'unknown')
            when = time.strftime('%H:%M, %d %b', time.localtime(self.checked)) \
                if self.checked else 'never'
            self.fact(g, 'Last check', when + (f' · offline' if self.check_error else ''),
                      'warn' if self.check_error else '')
            self.show_notes(r)
            if r.is_cached():
                self.fact(g, 'Package', f'cached · {human(os.path.getsize(r.cached))}', 'good')
                self.read_package(r)
            elif r.url and not self.busy:
                # Small (about a megabyte): fetch it now, to show what it holds.
                self.fact(g, 'Package', f'fetching · {human(r.size)}')
                self.in_thread(lambda: download(r), lambda _: self.on_release(user=False),
                               lambda e: (self.fact(g, 'Package', f'not fetched: {e}', 'bad'),
                                          self.update_all()))
            self.update_all()

        def read_package(self, r):
            try:
                self.package = Package(r.cached)
            except Exception as e:
                self.fact(self.g_rel, 'Package', f'unreadable: {e}', 'bad')
                self.package = None
                return
            p = self.package
            self.fact(self.g_rel, 'Schema', f'{p.schema}' + ('' if p.supported() else
                      ' — needs a newer installer'), '' if p.supported() else 'bad')

        def show_notes(self, r):
            buf = self.notes.get_buffer()
            buf.set_text('')
            text = r.notes or 'No notes for this release.'
            text = text.split('\n---\n')[0]
            for line in text.splitlines():
                end = buf.get_end_iter()
                if line.startswith('### '):
                    buf.insert_with_tags_by_name(end, line[4:].upper() + '\n', 'h3')
                elif line.startswith('## ') or line.startswith('# '):
                    buf.insert_with_tags_by_name(end, line.lstrip('# ').upper() + '\n', 'h')
                else:
                    if line.startswith('- '):
                        buf.insert_with_tags_by_name(end, '▸ ', 'bullet')
                        line = line[2:]
                    for i, part in enumerate(line.split('`')):
                        if i % 2:
                            buf.insert_with_tags_by_name(buf.get_end_iter(), part, 'code')
                        else:
                            buf.insert(buf.get_end_iter(), part)
                    buf.insert(buf.get_end_iter(), '\n')

        # ---------------------------------------------------------- what it installs
        def update_all(self):
            self.update_layers()
            self.update_launcher()
            self.update_banner()
            self.update_buttons()
            r = self.release()
            self.footer.set_label(f'PACKAGE SCHEMA {"/".join(map(str, SCHEMAS))}  ·  '
                                  f'CACHE {short(CACHE)}' + (f'  ·  RELEASE {r.version}' if r else ''))

        def update_layers(self):
            box = self.layers_box
            while (c := box.get_first_child()):
                box.remove(c)
            p, f = self.package, getattr(self, 'facts', None)
            if not p:
                box.append(lab('The release package is not here yet.', ('prose',)))
                return
            colors = ['orange', 'peach', 'blue', 'lilac', 'tan', 'sky', 'gold']
            later = []
            for i, l in enumerate(p.layers):
                if l['id'] in EXCLUDED:
                    later.append(l)
                    continue
                yes, why = layer_verdict(l, f) if f else (True, 'will install')
                box.append(self.layer_row(l['name'], l.get('version', ''), l.get('summary', ''),
                                          why, 'yes' if yes else 'no', colors[i % len(colors)]))
            # What this installer does not install, for now: shown, and switched off.
            for l in later + p.not_included:
                row = self.layer_row(l['name'], '', l['summary'], 'not available yet', 'na', None)
                sw = Gtk.Switch(active=False, sensitive=False, valign=Gtk.Align.CENTER)
                row.append(sw)
                box.append(row)

        def layer_row(self, name, version, summary, verdict, kind, color):
            row = Gtk.Box(spacing=14)
            row.add_css_class('layer-row')
            chip = Gtk.Box()
            chip.add_css_class('chip')
            chip.add_css_class('c-' + color if color else 'off')
            chip.set_valign(Gtk.Align.CENTER)
            chip.set_hexpand(False)
            chip.set_size_request(176, -1)
            chip.append(lab(name.upper(), xalign=1, hexpand=True))
            row.append(chip)
            row.append(lab(version, ('lver',)))
            s = lab(summary, ('prose',), wrap=True, width_chars=24, max_width_chars=70)
            s.set_hexpand(True)
            row.append(s)
            v = lab(verdict.upper(), ('verdict', kind), xalign=1, wrap=True,
                    width_chars=16, max_width_chars=26)
            row.append(v)
            return row

        def wanted_env(self):
            f = getattr(self, 'facts', None)
            if not f:
                return {}
            st = f.get('state') or {}
            if st.get('launcher_env') and st.get('version') == (self.package.version if self.package else None):
                return st['launcher_env']
            return launch_env(f, self.package)

        def update_launcher(self):
            g = self.game()
            env = self.wanted_env()
            text = '\n'.join(f'{k}={v}' for k, v in env.items())
            self.env_text.get_buffer().set_text(text or '(choose the game first)')
            grid = self.g_env
            while (c := grid.get_first_child()):
                grid.remove(c)
            if not g or not g.heroic:
                self.sw_heroic.set_sensitive(False)
                self.heroic_label.set_label('No Heroic entry for this install: set the '
                                            'variables below in your launcher.')
                return
            self.sw_heroic.set_sensitive(True)
            changes, conflicts = heroic_plan(g.heroic, env)
            on = self.sw_heroic.get_active()
            msg = f'Write them to {g.heroic.label()} (GOG app {g.heroic.app})'
            if changes is None:
                msg += ' — ' + conflicts[0]
            elif not changes:
                msg += ' — already set'
            if on and heroic_running():
                msg += '. Heroic is running: restart it after installing'
            self.heroic_label.set_label(msg)
            for j, h in enumerate(('VARIABLE', 'NOW', 'AFTER INSTALL')):
                grid.attach(lab(h, ('key',)), j, 0, 1, 1)
            cur = {}
            data = load_json(g.heroic.config) or {}
            for e in (data.get(g.heroic.app) or {}).get('enviromentOptions', []):
                if isinstance(e, dict):
                    cur[e.get('key')] = e.get('value', '')
            ch = {c['key']: c['new'] for c in (changes or [])}
            for i, (k, v) in enumerate(env.items(), 1):
                now = cur.get(k, '(unset)')
                after = ch.get(k, now) if on else now
                grid.attach(lab(k, ('val',)), 0, i, 1, 1)
                grid.attach(lab(now, ('val',), wrap=True), 1, i, 1, 1)
                grid.attach(lab(after, ('val', 'good') if k in ch and on else ('val',), wrap=True),
                            2, i, 1, 1)
            for i, c in enumerate(conflicts or [], len(env) + 1):
                grid.attach(lab(c, ('val', 'warn'), wrap=True), 0, i, 3, 1)

        def update_banner(self):
            p = self.package
            newer = None
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
                self.banner_text.set_label(f'Release {p.version} uses package schema '
                                           f'{p.schema}, which this installer cannot read. '
                                           'Download the newest installer.')
            elif newer:
                self.banner_text.set_label(f'A newer installer ({newer[0]}) comes with '
                                           f'release {newer[1].version}.')
            else:
                self.banner.set_visible(False)
                return
            r = newer[1] if newer else self.release()
            self.banner_uri = (r.installer_url if r and r.installer_url else
                               r.page if r else RELEASES_PAGE)
            self.banner.set_visible(True)

        def update_buttons(self):
            g, r, p = self.game(), self.release(), self.package
            f = getattr(self, 'facts', None)
            inst = f and f['installed']
            ok = bool(g and r and (p is None or p.supported()) and (p or r.url))
            self.b_install.set_sensitive(not self.busy and ok)
            self.b_uninstall.set_sensitive(not self.busy and bool(g and inst))
            self.b_refresh.set_sensitive(not self.busy)
            label = 'Engage'
            if r and inst:
                label = ('Reinstall ' if inst == r.version else 'Install ' if inst == '?' else
                         'Upgrade to ' if vtuple(r.version) > vtuple(inst)
                         else 'Switch to ') + r.version
            elif r:
                label = f'Install {r.version}'
            self.b_install.get_child().set_label(label.upper())

        def copy_env(self):
            buf = self.env_text.get_buffer()
            text = buf.get_text(buf.get_start_iter(), buf.get_end_iter(), False)
            self.get_clipboard().set(text)
            self.set_status('Copied to the clipboard', 'ok')

        # ---------------------------------------------------------- jobs
        def report(self, frac, step, info):
            def ui():
                text = STATUS.get(step, step)
                text = text.format(version=(info or {}).get('version', ''))
                if info and info.get('detail'):
                    text += f' · {info["detail"]}'
                self.set_status(text + ('…' if frac < 1 else ''))
                self.bar.set(frac)
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
            if not g or not r:
                return
            if game_running():
                self.set_status('Armada II is running — quit the game first', 'err')
                return
            write = self.sw_heroic.get_active() and bool(g.heroic)
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
            self.bar.set(0.0, busy=True)
            self.nav['log'].get_active() or None
            self.log(f'== install {r.version} into {g.path}')

            def done(res):
                self.set_busy(False)
                self.bar.set(1.0, busy=False)
                msg = f'Refit {res["version"]} installed'
                if write and res['heroic']:
                    msg += ' · Heroic launch settings written'
                elif res['env']:
                    msg += ' · set the launch variables (03-Launcher)'
                self.set_status(msg, 'ok')
                self.detail.set_label(' · '.join(res['notes']))
                self.on_game()
            self.in_thread(lambda: self.job().install(r, None, g.path, launcher=write), done)

        def on_uninstall(self):
            g = self.game()
            if not g:
                return
            self.confirm('Uninstall Armada II Refit?',
                         f'Everything the mod put into {short(g.path)} is taken out and the '
                         'stock files put back. Launch settings written to Heroic are taken '
                         'out again.', 'Uninstall', lambda: self.start_uninstall(g),
                         destructive=True)

        def start_uninstall(self, g):
            self.set_busy(True)
            self.bar.set(0.0, busy=True)
            self.log(f'== uninstall from {g.path}')

            def done(_):
                self.set_busy(False)
                self.bar.set(1.0, busy=False)
                self.set_status('Uninstalled · the game is stock again', 'ok')
                self.on_game()
            self.in_thread(lambda: self.job().uninstall(self.releases, g.path), done)

        def fetch_font(self):
            try:
                p = font_path(fetch=True)
            except Exception:
                return
            if p:
                GLib.idle_add(lambda: (add_font(p), self.restyle()))

        def restyle(self):
            # Pango caches the face per widget; nudging every label's style re-resolves it.
            def walk(w):
                if isinstance(w, Gtk.Label):
                    w.add_css_class('refont')
                    w.remove_css_class('refont')
                c = w.get_first_child()
                while c:
                    walk(c)
                    c = c.get_next_sibling()
            load_css()
            walk(self)
            self.update_all()

    provider = Gtk.CssProvider()

    def load_css():
        if hasattr(provider, 'load_from_string'):
            provider.load_from_string(CSS)
        else:
            provider.load_from_data(CSS.encode(), -1)

    def activate(app):
        Adw.StyleManager.get_default().set_color_scheme(Adw.ColorScheme.FORCE_DARK)
        load_css()
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
              'Install them (Arch: python-gobject gtk4 libadwaita; Debian/Ubuntu: '
              'python3-gi gir1.2-gtk-4.0 gir1.2-adw-1; Fedora: python3-gobject gtk4 '
              'libadwaita), or use the command line: --help.', file=sys.stderr)
        sys.exit(1)
