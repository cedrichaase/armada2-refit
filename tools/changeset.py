#!/usr/bin/env python3
"""Changesets: how this project records changes and makes releases (CLAUDE.md,
"Changelogs and versions"; .changeset/README.md for the file format).

A change adds a small file in .changeset/ that names the layers it touches, the games
whose version track it belongs to, the bump each layer takes and what changed. Nobody
edits a version by hand: `version --game G` (run by the release workflow for the default
game, or by hand) consumes the pending changesets naming G, computes each layer's next
version on G's track as its current one plus the largest bump among them, and the
project's (the root's) as its current one plus the largest bump of all.

The default game's track (games/<id>.toml `default = true`, Armada II) is the one players
get: each layer's CHANGELOG.md entry, the root entry listing the layer versions it
bundles, every other place the version lives (Cargo.toml, .changeset/config.toml). Every
other game's track lives in one file, games/<id>/CHANGELOG.md, and nowhere else. A
consumed changeset loses that game from its `games:`, and is deleted when none is left.

    changeset.py add [--layers a,b --bump minor | --set a=minor ...] [--games g,h]
                     [--section Fixed] [--status TEXT] [--name NAME] [-m TEXT]
                                                         (interactive without -m)
    changeset.py status [--game G] [--json | --markdown | --next-root]
    changeset.py version [--game G] [--date YYYY-MM-DD] [--no-hash] [--dry-run] [--commit]
    changeset.py released RANGE   (the version a release commit in RANGE carries, if any)
    changeset.py check [--base REF [--allow-version-change]]
    changeset.py convert [--base REF] [--dry-run]   (hand-numbered entries -> changesets)

--root DIR runs against another checkout (the self-test's fixtures). Python 3.11 or
newer, standard library only. Exits 1 on a refused changeset or a failed check.
"""
import argparse
import datetime
import json
import os
import re
import secrets
import subprocess
import sys
import tomllib

BUMPS = ('patch', 'minor', 'major')
SECTIONS = ('Added', 'Changed', 'Fixed', 'Removed')
DEFAULT_SECTION = 'Changed'
DEFAULT_STATUS = 'Installed, not yet seen in game'
ROOT_ID = 'root'
RESERVED = ('layers', 'bump', 'status', 'games')
STATUS_FOR = 'status.'   # status.<game>: a status for one game's track
HEAD_RE = re.compile(r'^## (\d+)\.(\d+)\.(\d+) — (.*)$')
STATUS_RE = re.compile(r'(Installed, not yet seen in game|Confirmed in game(?: \d{4}-\d{2}-\d{2})?)\.?')


class Refused(Exception):
    pass


# ---------------------------------------------------------------- games
#
# games/<id>.toml (games/README.md): each game has a version track of its own. The
# default game's is the one the version files and the published changelogs carry.

class Game:
    def __init__(self, gid, desc):
        self.id = gid
        self.label = desc.get('label', gid)
        self.offered = bool(desc.get('offered'))
        self.default = bool(desc.get('default'))
        # What a changeset on an offered game's track must not name: the id, the label
        # and the descriptor's [changesets] quiet words (`check`).
        self.quiet = [gid, self.label] + list(desc.get('changesets', {}).get('quiet', []))

    @property
    def changelog(self):
        return None if self.default else os.path.join('games', self.id, 'CHANGELOG.md')


def load_games(root):
    """{id: Game}, the default first, the others by id."""
    d = os.path.join(root, 'games')
    out = {}
    for f in sorted(os.listdir(d)) if os.path.isdir(d) else ():
        if f.endswith('.toml'):
            with open(os.path.join(d, f), 'rb') as fh:
                out[f[:-5]] = Game(f[:-5], tomllib.load(fh))
    defaults = [g for g in out.values() if g.default]
    if len(defaults) != 1:
        raise Refused(f'games/: {len(defaults)} descriptors are marked default, not one')
    return dict(sorted(out.items(), key=lambda kv: not kv[1].default))


def default_game(games):
    return next(iter(games))


# ---------------------------------------------------------------- units

class Unit:
    """Something with its own version: a layer of layers.toml, a unit of
    .changeset/config.toml, or the project itself (root)."""

    def __init__(self, uid, dir, layer, version_files=(), games=None):
        self.id, self.dir, self.layer = uid, dir, layer
        self.version_files = list(version_files)
        self.games = games   # a layer's `games` (layers.toml); None: every game's

    @property
    def changelog(self):
        return os.path.join(self.dir, 'CHANGELOG.md') if self.dir else 'CHANGELOG.md'


def load_units(root):
    """{id: Unit}, in release order: the layers of layers.toml (retired ones and those
    without a CHANGELOG.md left out), then the units of .changeset/config.toml, then root."""
    with open(os.path.join(root, 'layers.toml'), 'rb') as fh:
        reg = tomllib.load(fh)
    cfg = {}
    p = os.path.join(root, '.changeset', 'config.toml')
    if os.path.exists(p):
        with open(p, 'rb') as fh:
            cfg = tomllib.load(fh)
    extra_files = {k: v.get('version_files', []) for k, v in cfg.get('version', {}).items()}
    units = {}
    for l in reg.get('layer', []):
        if l.get('select') == 'retired':
            continue
        d = l.get('dir', l['id'])
        if os.path.exists(os.path.join(root, d, 'CHANGELOG.md')):
            units[l['id']] = Unit(l['id'], d, True, extra_files.get(l['id'], ()), l.get('games'))
    for u in cfg.get('unit', []):
        units[u['id']] = Unit(u['id'], u.get('dir', u['id']), False, extra_files.get(u['id'], ()))
    units[ROOT_ID] = Unit(ROOT_ID, '', False, extra_files.get(ROOT_ID, ()))
    return units


def read(root, rel):
    with open(os.path.join(root, rel), encoding='utf-8') as fh:
        return fh.read()


def write(root, rel, text):
    with open(os.path.join(root, rel), 'w', encoding='utf-8') as fh:
        fh.write(text)


def changelog_version(text):
    for line in text.splitlines():
        m = HEAD_RE.match(line)
        if m:
            return tuple(int(x) for x in m.groups()[:3])
    return None


def current(root, unit):
    v = changelog_version(read(root, unit.changelog))
    if v is None:
        raise Refused(f'{unit.changelog}: no "## X.Y.Z — date" heading')
    return v


def vstr(v):
    return '.'.join(map(str, v))


def bumped(v, bump):
    if bump == 'major':
        return (v[0] + 1, 0, 0)
    if bump == 'minor':
        return (v[0], v[1] + 1, 0)
    return (v[0], v[1], v[2] + 1)


# ---------------------------------------------------------------- version files
#
# Where else a unit's version lives besides its CHANGELOG.md heading:
#   - <dir>/Cargo.toml's [package] version, and that package's entry in every Cargo.lock,
#     when it equals the changelog's (found by itself);
#   - [[version.<id>.version_files]] in .changeset/config.toml: a file and a regex whose
#     group "v" is the version (INSTALLER_VERSION in the GUI installer).

def cargo_sites(root, unit):
    """[(file, regex)] for the unit's Cargo package, or []."""
    if not unit.dir:
        return []
    toml = os.path.join(unit.dir, 'Cargo.toml')
    if not os.path.exists(os.path.join(root, toml)):
        return []
    text = read(root, toml)
    m = re.search(r'^\[package\]\n(?:(?!\[).*\n)*?name = "([^"]+)"', text, re.M)
    if not m:
        return []
    name = m.group(1)
    sites = [(toml, r'(?ms)^\[package\]\n(?:(?!\[)[^\n]*\n)*?version = "(?P<v>[^"]+)"')]
    for dirpath, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if d not in ('.git', 'target', 'build', '.claude', 'node_modules')]
        if 'Cargo.lock' in files:
            lock = os.path.relpath(os.path.join(dirpath, 'Cargo.lock'), root)
            if f'name = "{name}"\n' in read(root, lock):
                sites.append((lock, r'(?m)^name = "' + re.escape(name) + r'"\nversion = "(?P<v>[^"]+)"'))
    return sites


def version_sites(root, unit):
    sites = [(s['file'], s['pattern']) for s in unit.version_files]
    return cargo_sites(root, unit) + sites


def site_value(root, file, pattern):
    m = re.search(pattern, read(root, file))
    return m.group('v') if m else None


def set_site(root, file, pattern, new):
    text = read(root, file)
    m = re.search(pattern, text)
    if not m:
        raise Refused(f'{file}: no version matching {pattern!r}')
    write(root, file, text[:m.start('v')] + new + text[m.end('v'):])


# ---------------------------------------------------------------- changesets

def changeset_dir(root):
    return os.path.join(root, '.changeset')


def list_changesets(root):
    d = changeset_dir(root)
    if not os.path.isdir(d):
        return []
    return sorted(f for f in os.listdir(d) if f.endswith('.md') and f.lower() != 'readme.md')


def _scalar(s):
    s = s.strip()
    if len(s) >= 2 and s[0] == s[-1] and s[0] in '"\'':
        return s[1:-1]
    return s


def parse_frontmatter(text, name):
    """(dict, body). The frontmatter is the small YAML subset pnpm's changesets use:
    `key: value`, keys and values optionally quoted, and `key: [a, b]` lists."""
    lines = text.split('\n')
    if not lines or lines[0].strip() != '---':
        raise Refused(f'{name}: no frontmatter (the file starts with a "---" line)')
    try:
        end = next(i for i in range(1, len(lines)) if lines[i].strip() == '---')
    except StopIteration:
        raise Refused(f'{name}: the frontmatter has no closing "---"') from None
    fm = {}
    for n, line in enumerate(lines[1:end], 2):
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        m = re.match(r'^\s*("[^"]*"|\'[^\']*\'|[A-Za-z0-9_.-]+)\s*:\s*(.*?)\s*$', line)
        if not m:
            raise Refused(f'{name}:{n}: not "key: value": {line!r}')
        key, val = _scalar(m.group(1)), m.group(2)
        if val.startswith('['):
            if not val.endswith(']'):
                raise Refused(f'{name}:{n}: a list is written on one line, [a, b]')
            val = [_scalar(x) for x in val[1:-1].split(',') if x.strip()]
        else:
            val = _scalar(val)
        if key in fm:
            raise Refused(f'{name}:{n}: {key} given twice')
        fm[key] = val
    return fm, '\n'.join(lines[end + 1:]).strip('\n')


class Changeset:
    def __init__(self, name, bumps, status, body, games=(), statuses=None):
        self.name, self.bumps, self.status, self.body = name, bumps, status, body
        self.games = list(games)
        self.statuses = dict(statuses or {})   # status.<game> overrides
        self.bumps_on = {}   # {game: bumps}: the layers that are that game's
        self.hash = None
        self.order = (1, name)   # uncommitted: last

    def status_for(self, game):
        return self.statuses.get(game, self.status)

    def on(self, game):
        """This changeset as one game's track sees it: its status that game's."""
        c = Changeset(self.name, self.bumps_on.get(game, self.bumps), self.status_for(game),
                      self.body, self.games)
        c.hash, c.order = self.hash, self.order
        return c


def _status(v, name, key='status'):
    if not isinstance(v, str):
        raise Refused(f'{name}: {key} is one line of text')
    return '' if v.strip().lower() == 'none' else v.strip().rstrip('.')


def parse_changeset(text, name, units, games=None):
    """games: {id: Game}; None skips the games check (a fixture without games/)."""
    fm, body = parse_frontmatter(text, name)
    bumps = {}
    layers = fm.get('layers')
    if layers is not None:
        if isinstance(layers, str):
            layers = [layers]
        if not layers:
            raise Refused(f'{name}: layers is empty; a change to no layer is `layers: [root]`')
        if 'bump' not in fm:
            raise Refused(f'{name}: layers without a bump (bump: major, minor or patch)')
        if fm['bump'] not in BUMPS:
            raise Refused(f'{name}: bump {fm["bump"]!r} is not major, minor or patch')
        for l in layers:
            bumps[l] = fm['bump']
    elif 'bump' in fm:
        raise Refused(f'{name}: bump without layers (layers: [id, ...]; `[root]` for a change to no layer)')
    for k, v in fm.items():
        if k in RESERVED or k.startswith(STATUS_FOR):
            continue
        if not isinstance(v, str) or v not in BUMPS:
            raise Refused(f'{name}: {k}: {v!r} is not major, minor or patch')
        bumps[k] = v   # the per-layer form wins over `bump:`
    if not bumps:
        raise Refused(f'{name}: no bump (layers: [...] with bump:, or `<layer>: major|minor|patch`)')
    unknown = [k for k in bumps if k not in units]
    if unknown:
        raise Refused(f'{name}: no such layer: {", ".join(unknown)} '
                      f'(known: {", ".join(units)})')
    status = _status(fm.get('status', DEFAULT_STATUS), name)
    gs = fm.get('games')
    if gs is None:
        raise Refused(f'{name}: no games (games: [{default_game(games) if games else "<id>"}], the '
                      f'games whose version track it belongs to; games/<id>.toml)')
    if isinstance(gs, str):
        gs = [gs]
    if not gs:
        raise Refused(f'{name}: games is empty')
    if len(set(gs)) != len(gs):
        raise Refused(f'{name}: a game named twice in games')
    if games is not None:
        unknown = [g for g in gs if g not in games]
        if unknown:
            raise Refused(f'{name}: no such game: {", ".join(unknown)} (known: {", ".join(games)})')
        gs = [g for g in games if g in gs]   # the registry's order
    statuses = {}
    for k, v in fm.items():
        if k.startswith(STATUS_FOR):
            g = k[len(STATUS_FOR):]
            if g not in gs:
                raise Refused(f'{name}: {k}: {g} is not one of its games ({", ".join(gs)})')
            statuses[g] = _status(v, name, k)
    if not body.strip():
        raise Refused(f'{name}: no text under the frontmatter')
    for h in re.findall(r'^#{1,6} .*$', body, re.M):
        if not re.match(r'^### (%s)\s*$' % '|'.join(SECTIONS), h):
            raise Refused(f'{name}: heading {h!r}: only ### {", ### ".join(SECTIONS)}')
    c = Changeset(name, bumps, status, body, gs, statuses)
    if games is not None:
        # A layer that is not a game's (layers.toml `games`) takes no bump on its track.
        for g in gs:
            c.bumps_on[g] = {u: b for u, b in bumps.items()
                             if units[u].games is None or g in units[u].games}
            if not c.bumps_on[g]:
                raise Refused(f'{name}: names no unit of {g} ({", ".join(bumps)}: layers.toml '
                              f'games); take {g} off its games')
    return c


def git(root, *args):
    r = subprocess.run(['git', '-C', root, *args], stdout=subprocess.PIPE,
                       stderr=subprocess.DEVNULL, text=True)
    return r.stdout.strip() if r.returncode == 0 else None


def added(root):
    """{file name: (age rank, short hash)} of the commit that added each changeset, newest
    first in git's order (a timestamp ties within a second): rank 0 is the newest."""
    out = git(root, 'log', '--diff-filter=A', '--abbrev=7', '--format=@%h', '--name-only', '--', '.changeset')
    found, rank, h = {}, 0, None
    for line in (out or '').split('\n'):
        if line.startswith('@'):
            h, rank = line[1:], rank + 1
        elif line.strip() and h:
            found.setdefault(os.path.basename(line.strip()), (rank, h))
    return found


def load_changesets(root, units, games=None):
    out, errs = [], []
    adds = added(root)
    if games is None:
        games = load_games(root)
    for f in list_changesets(root):
        try:
            cs = parse_changeset(read(root, os.path.join('.changeset', f)), f, units, games)
        except Refused as e:
            errs.append(str(e))
            continue
        # The commit that added it: its place in the release (oldest first), and the hash
        # the entry cites. A changeset not committed yet comes last.
        if f in adds:
            rank, cs.hash = adds[f]
            cs.order = (-rank, f)
        out.append(cs)
    out.sort(key=lambda c: c.order)
    return out, errs


# ---------------------------------------------------------------- the plan

LAYERS_RE = re.compile(r'([A-Za-z0-9_.-]+) (\d+)\.(\d+)\.(\d+)')


def track_versions(root, game):
    """{unit id: version} on another game's track, from games/<id>/CHANGELOG.md: root is
    the newest heading's, each unit the newest `Layers:` line that names it. A unit not
    released on the track yet is absent (it starts from 0.0.0)."""
    p = os.path.join(root, game.changelog)
    out = {}
    if not os.path.exists(p):
        return out
    text = read(root, game.changelog)
    v = changelog_version(text)
    if v:
        out[ROOT_ID] = v
    for line in text.splitlines():
        if line.startswith('Layers: '):
            for m in LAYERS_RE.finditer(line[len('Layers: '):]):
                out.setdefault(m.group(1), tuple(int(x) for x in m.groups()[1:]))
    return out


def for_game(changesets, game):
    """The changesets naming the game, each with that game's status."""
    if game is None:
        return list(changesets)
    return [c.on(game.id) for c in changesets if game.id in c.games]


def plan(root, units, changesets, game=None):
    """{id: dict(current, next, bump, changesets=[(name, bump)])} for every unit a
    changeset names, and root, which takes the largest bump of all. game (a Game): only
    the changesets naming it, and its track's versions; None: every changeset, on the
    version files' track (the default game's)."""
    changesets = for_game(changesets, game)
    track = track_versions(root, game) if game is not None and not game.default else None
    out = {}
    for uid, unit in units.items():
        named = [(c.name, c.bumps[uid]) for c in changesets if uid in c.bumps]
        if uid == ROOT_ID:
            named_all = [(c.name, max(c.bumps.values(), key=BUMPS.index)) for c in changesets]
            if not named_all:
                continue
            named = named_all
        elif not named:
            continue
        bump = max((b for _, b in named), key=BUMPS.index)
        cur = current(root, unit) if track is None else track.get(uid, (0, 0, 0))
        out[uid] = dict(current=vstr(cur), next=vstr(bumped(cur, bump)), bump=bump,
                        changesets=named)
    return out


# ---------------------------------------------------------------- entries

def split_body(body):
    """(preamble, {section: text}) of a changeset's body, sections in SECTIONS order;
    text without a heading is DEFAULT_SECTION's when no heading follows it, else the
    preamble."""
    parts = re.split(r'^### (\w+)\s*$', body, flags=re.M)
    pre, secs = parts[0].strip('\n'), {}
    for i in range(1, len(parts), 2):
        txt = parts[i + 1].strip('\n')
        if txt.strip():
            secs[parts[i]] = (secs[parts[i]] + '\n' + txt) if parts[i] in secs else txt
    if pre.strip() and not secs:
        return '', {DEFAULT_SECTION: pre}
    return pre, secs


def with_tail(text, tail):
    """text with tail appended to its last line."""
    if not tail:
        return text
    lines = text.rstrip('\n').split('\n')
    last = lines[-1].rstrip()
    if last.endswith(('.', '!', '?', ')', '`')):
        lines[-1] = f'{last} {tail}'
    else:
        lines[-1] = f'{last}. {tail}'
    return '\n'.join(lines)


def prefixed(text, prefix):
    """Every top-level bullet of text (or its first line) led by `prefix: `."""
    if not prefix:
        return text
    lines = text.split('\n')
    bullets = [i for i, l in enumerate(lines) if l.startswith('- ')]
    for i in bullets or [0]:
        if lines[i].startswith('- '):
            lines[i] = f'- {prefix}: {lines[i][2:]}'
        else:
            lines[i] = f'{prefix}: {lines[i]}'
    return '\n'.join(lines)


def compose(version, date, items, layers_line=None, preambles=()):
    """An entry: items is [(changeset, section texts, prefix)]."""
    statuses = {c.status for c, _, _ in items}
    uniform = len(statuses) == 1
    out = [f'## {version} — {date}', '']
    if layers_line:
        out += [layers_line, '']
    for p in preambles:
        out += [p, '']
    for sec in SECTIONS:
        blocks = []
        for c, secs, prefix in items:
            if sec not in secs:
                continue
            txt = prefixed(secs[sec], prefix)
            tail = f'`{c.hash}`' if c.hash and c.hash not in txt else ''
            txt = with_tail(txt, tail)
            if not uniform and c.status:
                txt = with_tail(txt, c.status + '.')
            blocks.append(txt)
        if blocks:
            out += [f'### {sec}'] + blocks + ['']
    if uniform and next(iter(statuses)):
        out += [next(iter(statuses)) + '.', '']
    return '\n'.join(out)


def insert_entry(text, entry):
    """The entry before the changelog's newest one (after its header)."""
    lines = text.split('\n')
    at = next((i for i, l in enumerate(lines) if l.startswith('## ')), len(lines))
    head = '\n'.join(lines[:at]).rstrip('\n')
    rest = '\n'.join(lines[at:])
    if not rest.strip():   # the first entry
        return head + '\n\n' + entry.rstrip('\n') + '\n'
    return head + '\n\n' + entry.rstrip('\n') + '\n\n' + rest.lstrip('\n')


def layers_line(units, p):
    lay = [f'{u} {p[u]["next"]}' for u in units if u in p and units[u].layer]
    oth = [f'{u} {p[u]["next"]}' for u in units if u in p and u != ROOT_ID and not units[u].layer]
    if not lay and not oth:
        return None
    return 'Layers: ' + '; '.join(x for x in (', '.join(lay), ', '.join(oth)) if x) + '.'


def apply(root, units, changesets, date, hashes=True, game=None):
    """Write the release of one game's track (None: every changeset, on the default
    track): entries, version files (the default track only), and the consumed changesets
    taken off that game (deleted when no game is left). Returns the plan."""
    p = plan(root, units, changesets, game)
    if not p:
        raise Refused(f'no pending changesets{f" for {game.id}" if game else ""}')
    consumed = changesets
    changesets = for_game(changesets, game)
    if not hashes:
        for c in changesets:
            c.hash = None
    if game is not None and not game.default:
        apply_track(root, units, changesets, date, game, p)
        consume(root, consumed, game)
        return p
    for uid, unit in units.items():
        if uid not in p or uid == ROOT_ID:
            continue
        items, pre = [], []
        for c in changesets:
            if uid in c.bumps:
                pr, secs = split_body(c.body)
                if pr:
                    pre.append(pr)
                items.append((c, secs, None))
        entry = compose(p[uid]['next'], date, items, preambles=pre)
        write(root, unit.changelog, insert_entry(read(root, unit.changelog), entry))
    items = []
    for c in changesets:
        named = [u for u in units if u in c.bumps and u != ROOT_ID]
        items.append((c, split_body(c.body)[1], ', '.join(named)))
    root_unit = units[ROOT_ID]
    entry = compose(p[ROOT_ID]['next'], date, items, layers_line(units, p))
    write(root, root_unit.changelog, insert_entry(read(root, root_unit.changelog), entry))
    for uid in p:
        for file, pat in version_sites(root, units[uid]):
            if site_value(root, file, pat) == p[uid]['current']:
                set_site(root, file, pat, p[uid]['next'])
    consume(root, consumed, game)
    return p


def consume(root, changesets, game):
    """Each changeset naming the game loses it from `games:` (and its `status.<game>:`),
    and is deleted when it names no game any more. game None: every changeset deleted."""
    for c in changesets:
        path = os.path.join(changeset_dir(root), c.name)
        if game is None or c.games == [game.id]:
            os.remove(path)
            continue
        if game.id not in c.games:
            continue
        text = read(root, os.path.join('.changeset', c.name))
        end = text.index('\n---', 3)
        fm, rest = text[:end], text[end:]
        left = [g for g in c.games if g != game.id]
        fm = re.sub(r'(?m)^games\s*:.*$', f'games: [{", ".join(left)}]', fm, count=1)
        fm = re.sub(r'(?m)^["\']?' + re.escape(STATUS_FOR + game.id) + r'["\']?\s*:.*\n?', '', fm)
        write(root, os.path.join('.changeset', c.name), fm + rest)


def game_changelog_header(game):
    return (f'# Changelog — {game.label}\n\n'
            f'The version track of {game.label} (`{game.id}`): what changed for it, and each '
            f'layer\'s version on this track. Written by `tools/changeset.py version --game '
            f'{game.id}`, run by hand, from the changesets that name `{game.id}` in their '
            f'`games:` (`.changeset/README.md`, "Games"). Never published as a release. '
            f'Newest first.\n')


def apply_track(root, units, changesets, date, game, p):
    """Another game's release: one entry in games/<id>/CHANGELOG.md, with a Layers line
    giving every unit's version on the track and the items grouped by layer."""
    rel = game.changelog
    if not os.path.exists(os.path.join(root, rel)):
        os.makedirs(os.path.dirname(os.path.join(root, rel)), exist_ok=True)
        write(root, rel, game_changelog_header(game))
    track = track_versions(root, game)
    full = {u: dict(next=vstr(v)) for u, v in track.items() if u in units}
    full.update({u: e for u, e in p.items()})
    order = list(units)

    def first(c):
        named = [u for u in units if u in c.bumps and u != ROOT_ID]
        return order.index(named[0]) if named else len(order)
    items = []
    for c in sorted(changesets, key=first):   # stable: oldest first within a layer
        named = [u for u in units if u in c.bumps and u != ROOT_ID]
        items.append((c, split_body(c.body)[1], ', '.join(named)))
    entry = compose(p[ROOT_ID]['next'], date, items, layers_line(units, full))
    write(root, rel, insert_entry(read(root, rel), entry))


# ---------------------------------------------------------------- check

def quiet_hits(c, games):
    """What a changeset on an offered game's track says of a game that is not offered
    (games/README.md, "Not offered, not released"): [(game, word)]."""
    if not any(games[g].offered for g in c.games):
        return []
    hits = []
    for g in games.values():
        if g.offered:
            continue
        for w in g.quiet:
            if re.search(r'(?<![A-Za-z0-9])' + re.escape(w), c.body):
                hits.append((g.id, w))
    return hits


def shipped_files(root):
    """What a player or a reader of the published pages receives, from publish/package.sh and
    layers.toml, not a hand list: every repository file package.sh names under $root/ and
    every glob among them, the guides, the installers' sources, the GUI installer, the
    settings tables (their labels and help are drawn on the Refit screen) and the changelogs
    of the offered games' layers. Developer material (the layer READMEs, profiles, plans,
    defaults of a game) is not among them."""
    import glob
    out = set()
    pk = read(root, 'publish/package.sh') if os.path.exists(os.path.join(root, 'publish/package.sh')) else ''
    # The cp statements only (with their continuation lines): the rest of package.sh
    # runs repository tools (tools/layers.py, games/) that no package carries.
    stmt, cps = '', []
    for line in pk.splitlines():
        stmt += line + ' '
        if line.rstrip().endswith('\\'):
            stmt = stmt.rstrip('\\ ') + ' '
            continue
        if re.match(r'\s*cp ', stmt):
            cps.append(stmt)
        stmt = ''
    for st in cps:
        for m in re.finditer(r'\$root/([A-Za-z0-9_./*-]+)', st):
            for f in glob.glob(os.path.join(root, m.group(1))):
                if os.path.isfile(f):
                    out.add(os.path.relpath(f, root))
    for d in ('publish/installer', 'gui-installer', 'csdk'):
        for dp, _, fs in os.walk(os.path.join(root, d)):
            if 'build' in dp.split(os.sep):
                continue
            out.update(os.path.relpath(os.path.join(dp, f), root) for f in fs)
    out.update(('README.md', 'PLAYING.md', 'MODDING.md', 'CHANGELOG.md', 'publish/MODDING.txt',
                'retire/known.txt'))
    for dp, _, fs in os.walk(root):
        rel = os.path.relpath(dp, root)
        if rel.split(os.sep)[0] in ('.git', 'games', 'target', 'plans', '.claude'):
            continue
        for f in fs:
            if f in ('settings.toml', 'CHANGELOG.md') and rel.count(os.sep) < 1:
                out.add(os.path.normpath(os.path.join(rel, f)))
    out = {f for f in out if not f.endswith(('.png', '.tga', '.dll', '.asi', '.exe', '.lib'))}
    return sorted(f for f in out if os.path.exists(os.path.join(root, f)))


def shipped_hits(root, games):
    """[(file, line, game, word)]: a word of a game that is not offered, in a file a player
    receives (publish/README.md, "Nothing of a quiet game in what players read")."""
    words = [(g.id, w) for g in games.values() if not g.offered for w in g.quiet]
    res = [(gid, w, re.compile(r'(?<![A-Za-z0-9])' + re.escape(w))) for gid, w in words]
    hits = []
    for f in shipped_files(root):
        try:
            text = read(root, f)
        except (UnicodeDecodeError, OSError):
            continue
        for n, line in enumerate(text.splitlines(), 1):
            for gid, w, rx in res:
                if rx.search(line):
                    hits.append((f, n, gid, w))
    return hits


def check(root, units, base=None, allow_version_change=False):
    errs = []
    games = load_games(root)
    cs, perrs = load_changesets(root, units, games)
    errs += perrs
    for c in cs:
        for gid, w in quiet_hits(c, games):
            offered = ', '.join(g for g in c.games if games[g].offered)
            errs.append(f'{c.name}: names {w!r} ({gid}, not offered) on the track of {offered}, '
                        f'which is published: word it without it, or split off a changeset '
                        f'with games: [{gid}] (.changeset/README.md, "Games")')
    for f, n, gid, w in shipped_hits(root, games):
        errs.append(f'{f}:{n}: names {w!r} ({gid}, not offered) in a file players receive: '
                    f'reword it neutrally, or move it to developer material (publish/README.md)')
    for g in games.values():
        if g.changelog and os.path.exists(os.path.join(root, g.changelog)):
            for line in read(root, g.changelog).splitlines():
                if line.startswith('## ') and not HEAD_RE.match(line):
                    errs.append(f'{g.changelog}: {line!r} is not "## X.Y.Z — date"')
    for uid, unit in units.items():
        try:
            cur = vstr(current(root, unit))
        except Refused as e:
            errs.append(str(e))
            continue
        for file, pat in version_sites(root, unit):
            v = site_value(root, file, pat)
            if v is None and not file.endswith('Cargo.lock'):
                errs.append(f'{file}: no version for {uid} (pattern {pat!r})')
            elif v is not None and v != cur:
                errs.append(f'{file}: {uid} is {v}, its {unit.changelog} says {cur}')
    if base and not allow_version_change:
        for uid, unit in units.items():
            old = git(root, 'show', f'{base}:{unit.changelog}')
            if old is None:
                continue
            was, now = changelog_version(old), changelog_version(read(root, unit.changelog))
            if was and now and was != now:
                errs.append(f'{unit.changelog}: {vstr(was)} -> {vstr(now)} by hand; versions '
                            f'come from changesets (tools/changeset.py add), and only the '
                            f'release assigns them')
    return cs, errs


# ---------------------------------------------------------------- convert

def bump_between(old, new):
    if new[0] != old[0]:
        return 'major'
    if new[1] != old[1]:
        return 'minor'
    return 'patch'


def entries(text):
    """[(version tuple, start line, end line)] of a changelog's entries."""
    lines = text.split('\n')
    heads = [(i, HEAD_RE.match(l)) for i, l in enumerate(lines)]
    heads = [(i, m) for i, m in heads if m]
    out = []
    for k, (i, m) in enumerate(heads):
        end = heads[k + 1][0] if k + 1 < len(heads) else len(lines)
        out.append((tuple(int(x) for x in m.groups()[:3]), i, end))
    return out


def convert(root, units, base, dry_run=False):
    """Turn the hand-numbered entries this branch added since `base` into changesets:
    one per unit, its bump from base's version to the branch's newest, its body the
    entries' text with their status lines taken out (the status becomes the changeset's;
    "not yet seen" wins over "Confirmed"). The entries come out of the changelogs and the
    version files go back to base's version. The root's own entries are dropped when the
    branch has layer entries (they restate them) and become a root changeset when not."""
    made, notes = [], []
    branch = {}
    dflt = default_game(load_games(root))   # the changelogs are the default game's track
    for uid, unit in units.items():
        old = git(root, 'show', f'{base}:{unit.changelog}')
        if old is None:
            continue
        old_versions = {v for v, _, _ in entries(old)}
        text = read(root, unit.changelog)
        lines = text.split('\n')
        new = [(v, a, b) for v, a, b in entries(text) if v not in old_versions]
        if new:
            branch[uid] = (unit, old, lines, new)
    for uid, (unit, old, lines, new) in branch.items():
        if uid == ROOT_ID and len(branch) > 1:
            notes.append(f'CHANGELOG.md: dropped the root entries {", ".join(vstr(v) for v, _, _ in new)} '
                         f'(the layer entries carry their items); check nothing root-only is lost')
            body = None
        else:
            bodies, statuses, pre = {}, [], []
            for v, a, b in sorted(new):
                txt = '\n'.join(lines[a + 1:b]).strip('\n')
                txt = re.sub(r'^Layers:.*\n?', '', txt, flags=re.M)
                for s in STATUS_RE.findall(txt):
                    statuses.append(s.rstrip('.'))
                txt = re.sub(r'(?m)^\s*' + STATUS_RE.pattern + r'\s*$\n?', '', txt)
                txt = re.sub(r'(?m)[ \t]*' + STATUS_RE.pattern + r'[ \t]*$', '', txt)
                p, secs = split_body(txt)
                if p.strip():
                    pre.append(p.strip())
                for s, t in secs.items():
                    bodies.setdefault(s, []).append(t.strip('\n'))
            status = (DEFAULT_STATUS if not statuses or any('not yet' in s for s in statuses)
                      else sorted(statuses)[-1])
            body = '\n\n'.join(pre)
            for s in SECTIONS:
                if s in bodies:
                    body += ('\n\n' if body else '') + f'### {s}\n' + '\n'.join(bodies[s])
            was = changelog_version(old)
            top = max(v for v, _, _ in new)
            bump = bump_between(was, top)
            name = f'converted-{uid}.md'
            fm = (f'---\nlayers: [{uid}]\nbump: {bump}\ngames: [{dflt}]\nstatus: {status}\n---\n\n')
            made.append((name, fm + body.strip('\n') + '\n', uid, vstr(was), vstr(top), bump))
        # take the entries out, keep the rest
        drop = set()
        for _, a, b in new:
            drop.update(range(a, b))
        kept = '\n'.join(l for i, l in enumerate(lines) if i not in drop)
        kept = re.sub(r'\n{3,}', '\n\n', kept)
        if not dry_run:
            write(root, unit.changelog, kept)
            was = vstr(changelog_version(old))
            top = vstr(max(v for v, _, _ in new))
            for file, pat in version_sites(root, unit):
                if site_value(root, file, pat) == top:
                    set_site(root, file, pat, was)
    if not dry_run:
        os.makedirs(changeset_dir(root), exist_ok=True)
        for name, text, *_ in made:
            write(root, os.path.join('.changeset', name), text)
    return made, notes


# ---------------------------------------------------------------- the release commit
#
# `version --commit` commits the release as "Release X.Y.Z" with the trailer
# "Release-Version: X.Y.Z". CI publishes vX.Y.Z on a push to main exactly when that push
# brings in such a commit for the root version main now has (`released`): merging the
# release PR. A version changed by hand carries no trailer and publishes nothing.

TRAILER = 'Release-Version'


def commit_release(root, version):
    msg = f'Release {version}\n\n{TRAILER}: {version}\n'
    for args in (('add', '-A', '--', '.'), ('commit', '-q', '-m', msg)):
        r = subprocess.run(['git', '-C', root, *args])
        if r.returncode:
            raise Refused(f'git {args[0]} failed')


class Untrailed(Refused):
    """The root version changed in the range, but no commit carries its trailer."""


def released(root, rng):
    """The root version, when RANGE (BEFORE..HEAD) brings it in through a release commit:
    the root changelog's version differs from BEFORE's, and a commit in the range carries
    the trailer for it. None when the version did not change. Raises Untrailed when it
    changed without the trailer: a squash-merged release PR (the squash dropped the
    trailer) or a version changed by hand, which must not pass quietly."""
    ver = vstr(current(root, load_units(root)[ROOT_ID]))
    before = rng.split('..')[0]
    old = git(root, 'show', f'{before}:CHANGELOG.md')
    if old is not None and changelog_version(old) and vstr(changelog_version(old)) == ver:
        return None
    out = git(root, 'log', f'--format=%(trailers:key={TRAILER},valueonly)', rng)
    if out is not None and ver in {l.strip() for l in out.split('\n')}:
        return ver
    raise Untrailed(
        f'the root version became {ver} in {rng}, but no commit there carries '
        f'"{TRAILER}: {ver}". Was the release PR squash-merged? Merge it with a merge commit '
        f'or a rebase, never a squash. To publish {ver} now: dispatch CI on main with '
        f'`release` ticked (gh workflow run ci.yml --ref main -f release=true). A version '
        f'changed by hand is not a release: revert it and add a changeset.')


# ---------------------------------------------------------------- add

def slug(text):
    words = re.findall(r'[a-z0-9]+', text.lower())[:6]
    return '-'.join(words) or secrets.token_hex(3)


def add(root, units, bumps, status, section, message, name=None, games=None):
    gs = load_games(root)
    games = list(games or [default_game(gs)])
    if not bumps:
        raise Refused('no layer named (--layers a,b --bump minor, or --set a=minor)')
    for k, v in bumps.items():
        if k not in units:
            raise Refused(f'no such layer: {k} (known: {", ".join(units)})')
        if v not in BUMPS:
            raise Refused(f'{k}: {v!r} is not major, minor or patch')
    vals = set(bumps.values())
    if len(vals) == 1:
        fm = f'layers: [{", ".join(bumps)}]\nbump: {vals.pop()}\n'
    else:
        fm = ''.join(f'{k}: {v}\n' for k, v in bumps.items())
    fm += f'games: [{", ".join(games)}]\n'
    if status is not None:
        fm += f'status: {status}\n'
    msg = message.strip()
    if not msg.startswith(('- ', '### ')):
        msg = '- ' + msg
    if not msg.startswith('### '):
        msg = f'### {section}\n{msg}'
    os.makedirs(changeset_dir(root), exist_ok=True)
    base = name or slug(message)
    fn = base if base.endswith('.md') else base + '.md'
    if os.path.exists(os.path.join(changeset_dir(root), fn)):
        fn = fn[:-3] + '-' + secrets.token_hex(2) + '.md'
    text = f'---\n{fm}---\n\n{msg}\n'
    c = parse_changeset(text, fn, units, gs)   # refuse what check would
    hits = quiet_hits(c, gs)
    if hits:
        raise Refused(f'it names {", ".join(sorted({w for _, w in hits}))} on a published track '
                      f'(games: {", ".join(games)}): word it without, or --games '
                      f'{", ".join(sorted({g for g, _ in hits}))} alone')
    write(root, os.path.join('.changeset', fn), text)
    return os.path.join('.changeset', fn)


def ask(prompt, default=None):
    s = input(f'{prompt}{f" [{default}]" if default else ""}: ').strip()
    return s or default


# ---------------------------------------------------------------- main

def print_plan(p, units):
    if not p:
        print('No pending changesets: nothing to release.')
        return
    for uid in units:
        if uid not in p:
            continue
        e = p[uid]
        print(f'{uid:14} {e["current"]:>9} -> {e["next"]:<9} {e["bump"]}')
        if uid != ROOT_ID:
            for n, b in e['changesets']:
                print(f'{"":16}{b:6} {n}')
    print(f'{"":16}(root: the largest bump of the {len(p[ROOT_ID]["changesets"])} changeset(s))')


def markdown(p, units, changesets, game=None):
    if not p:
        return 'No unreleased changesets: this is the last release plus nothing.\n'
    out = ['| | released | next | bump |', '|---|---|---|---|']
    for uid in units:
        if uid in p:
            e = p[uid]
            out.append(f'| {uid} | {e["current"]} | {e["next"]} | {e["bump"]} |')
    out += ['', '**Unreleased changesets**', '']
    for c in for_game(changesets, game):
        lines = [l for l in c.body.split('\n') if l.strip() and not l.startswith('###')]
        first = next((l for l in lines if l.startswith('- ')), lines[0] if lines else '')
        first = first[2:] if first.startswith('- ') else first
        if len(first) > 160:
            first = first[:157] + '...'
        names = ', '.join(f'{k} {v}' for k, v in c.bumps.items())
        out.append(f'- `{c.name}` ({names}; {c.status or "no status"}): {first}')
    return '\n'.join(out) + '\n'


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--root', default=os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    sub = ap.add_subparsers(dest='cmd', required=True)
    a = sub.add_parser('add', help='write a changeset')
    a.add_argument('--layers', help='comma-separated layer ids (root for a change to no layer)')
    a.add_argument('--bump', choices=BUMPS)
    a.add_argument('--set', action='append', default=[], metavar='ID=BUMP',
                   help='a layer and its own bump (repeatable)')
    a.add_argument('--games', help='comma-separated game ids whose track it belongs to '
                   '(default: the default game, armada2)')
    a.add_argument('--section', choices=SECTIONS, default=None)
    a.add_argument('--status', help=f'the in-game status line (default "{DEFAULT_STATUS}"; "none" for none)')
    a.add_argument('--name')
    a.add_argument('-m', '--message')
    s = sub.add_parser('status', help='what the next release would bump')
    s.add_argument('--game', help='one game\'s track (default: every game; --json, --markdown '
                   'and --next-root: the default game\'s)')
    s.add_argument('--json', action='store_true')
    s.add_argument('--markdown', action='store_true')
    s.add_argument('--next-root', action='store_true',
                   help="print only the root version the next release gets (the current one plus a patch with none pending)")
    v = sub.add_parser('version', help='consume the changesets into a release')
    v.add_argument('--game', help='the track to release (default: the default game, armada2); '
                   'another game\'s only writes games/<id>/CHANGELOG.md')
    v.add_argument('--date', default=datetime.date.today().isoformat())
    v.add_argument('--no-hash', action='store_true', help='cite no commit hashes')
    v.add_argument('--dry-run', action='store_true', help='print the plan, write nothing')
    v.add_argument('--commit', action='store_true',
                   help=f'commit it as "Release X.Y.Z" with the {TRAILER} trailer CI publishes on')
    r = sub.add_parser('released', help='the version a release commit in RANGE brings to main')
    r.add_argument('range', help='BEFORE..HEAD')
    c = sub.add_parser('check', help='lint the changesets and the version files')
    c.add_argument('--base', help='fail on a version changed by hand since this ref')
    c.add_argument('--allow-version-change', action='store_true', help='the release PR')
    k = sub.add_parser('convert', help="turn a branch's hand-numbered entries into changesets")
    k.add_argument('--base', help='the ref to compare with (default: merge-base with origin/main)')
    k.add_argument('--dry-run', action='store_true')
    args = ap.parse_args(argv)
    root = os.path.abspath(args.root)
    units = load_units(root)
    try:
        games = load_games(root)

        def pick(gid):
            if gid is None:
                return games[default_game(games)]
            if gid not in games:
                raise Refused(f'no such game: {gid} (known: {", ".join(games)})')
            return games[gid]
        if args.cmd == 'add':
            bumps = {}
            if args.layers:
                if not args.bump:
                    raise Refused('--layers needs --bump')
                for l in args.layers.split(','):
                    bumps[l.strip()] = args.bump
            for kv in args.set:
                k_, _, v_ = kv.partition('=')
                bumps[k_.strip()] = v_.strip()
            msg, section, status = args.message, args.section or DEFAULT_SECTION, args.status
            if msg is None:
                if not sys.stdin.isatty():
                    raise Refused('-m TEXT, or run it in a terminal to be asked')
                print('Layers:', ', '.join(units))
                if not bumps:
                    ids = ask('Layers it changes (comma-separated)')
                    b = ask('Bump (major, minor, patch)', 'patch')
                    bumps = {x.strip(): b for x in (ids or '').split(',') if x.strip()}
                if not args.games:
                    print('Games:', ', '.join(games))
                    args.games = ask('Games whose track it belongs to (comma-separated)',
                                     default_game(games))
                section = ask(f'Section ({", ".join(SECTIONS)})', args.section or
                              ('Fixed' if set(bumps.values()) == {'patch'} else 'Added'))
                status = ask('In-game status ("none" for none)', status or DEFAULT_STATUS)
                msg = ask('What changed (one line; edit the file for more)')
                if not msg:
                    raise Refused('nothing written')
            gl = [g.strip() for g in (args.games or '').split(',') if g.strip()]
            if not gl:
                print(f'games: {default_game(games)} (the default; --games to change)', file=sys.stderr)
            print(add(root, units, bumps, status, section, msg, args.name, gl or None))
        elif args.cmd == 'status':
            cs, errs = load_changesets(root, units, games)
            if errs:
                raise Refused('\n'.join(errs))
            if args.next_root or args.json or args.markdown:
                # What CI publishes (the pre-release's version and notes, the release PR's
                # body): the default game's track, unless --game says otherwise.
                g = pick(args.game)
                p = plan(root, units, cs, g)
                if args.next_root:
                    # With nothing pending, the next is at least a patch: a pre-release
                    # X.Y.Z-pre.N must sort above the release X.Y.Z it follows.
                    cur = (current(root, units[ROOT_ID]) if g.default
                           else track_versions(root, g).get(ROOT_ID, (0, 0, 0)))
                    print(p[ROOT_ID]['next'] if p else vstr(bumped(cur, 'patch')))
                elif args.json:
                    print(json.dumps(dict(game=g.id, plan=p,
                                          changesets=[c.name for c in for_game(cs, g)]), indent=1))
                else:
                    sys.stdout.write(markdown(p, units, cs, g))
            else:
                for g in ([pick(args.game)] if args.game else list(games.values())):
                    where = 'the version files' if g.default else g.changelog
                    print(f'== {g.id} ({where})')
                    print_plan(plan(root, units, cs, g), units)
        elif args.cmd == 'version':
            g = pick(args.game)
            if args.commit and not g.default:
                # Only the default game's track is released: the commit's trailer is what CI
                # publishes a GitHub release on. Another game's changelog is committed by
                # hand, like any change.
                raise Refused(f'--commit releases the default game only ({default_game(games)}); '
                              f'{g.id} is {"not offered and never" if not g.offered else "not"} '
                              f'released on GitHub. Run it without --commit and commit '
                              f'{g.changelog} by hand')
            cs, errs = load_changesets(root, units, games)
            if errs:
                raise Refused('\n'.join(errs))
            if args.dry_run:
                print_plan(plan(root, units, cs, g), units)
                return 0
            p = apply(root, units, cs, args.date, hashes=not args.no_hash, game=g)
            print_plan(p, units)
            if args.commit:
                commit_release(root, p[ROOT_ID]['next'])
            print(p[ROOT_ID]['next'] if g.default else f'{g.changelog}: {p[ROOT_ID]["next"]}')
        elif args.cmd == 'released':
            v_ = released(root, args.range)   # Untrailed is a Refused: exit 1
            if v_:
                print(v_)
        elif args.cmd == 'check':
            cs, errs = check(root, units, args.base, args.allow_version_change)
            for e in errs:
                print('changeset check:', e, file=sys.stderr)
            if errs:
                return 1
            print(f'{len(cs)} pending changeset(s), {len(units)} units, {len(load_games(root))} games: ok')
        elif args.cmd == 'convert':
            base = args.base or git(root, 'merge-base', 'HEAD', 'origin/main')
            if not base:
                raise Refused('no --base, and no merge-base with origin/main')
            made, notes = convert(root, units, base, args.dry_run)
            for name, text, uid, was, top, bump in made:
                print(f'{".changeset/" + name}: {uid} {was} -> {top} becomes a {bump} bump')
            for n in notes:
                print('note:', n)
            if not made:
                print(f'nothing to convert: no entry here that {base} lacks')
    except Refused as e:
        print(f'changeset: {e}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
