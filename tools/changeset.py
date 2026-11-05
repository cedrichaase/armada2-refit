#!/usr/bin/env python3
"""Changesets: how this project records changes and makes releases (CLAUDE.md,
"Changelogs and versions"; .changeset/README.md for the file format).

A change adds a small file in .changeset/ that names the layers it touches, the bump
each takes and what changed. Nobody edits a version by hand: `version` (run by the
release workflow, or locally) consumes every pending changeset, computes each layer's
next version as its current one plus the largest bump among the changesets naming it,
and the project's (the root's) as its current one plus the largest bump of all; then it
writes each layer's CHANGELOG.md entry, the root entry listing the layer versions it
bundles, every other place the version lives (.changeset/config.toml), and deletes the
consumed files.

    changeset.py add [--layers a,b --bump minor | --set a=minor ...] [--section Fixed]
                     [--status TEXT] [--name NAME] [-m TEXT]   (interactive without -m)
    changeset.py status [--json | --markdown | --next-root]
    changeset.py version [--date YYYY-MM-DD] [--no-hash] [--dry-run]
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
RESERVED = ('layers', 'bump', 'status')
HEAD_RE = re.compile(r'^## (\d+)\.(\d+)\.(\d+) — (.*)$')
STATUS_RE = re.compile(r'(Installed, not yet seen in game|Confirmed in game(?: \d{4}-\d{2}-\d{2})?)\.?')


class Refused(Exception):
    pass


# ---------------------------------------------------------------- units

class Unit:
    """Something with its own version: a layer of layers.toml, a unit of
    .changeset/config.toml, or the project itself (root)."""

    def __init__(self, uid, dir, layer, version_files=()):
        self.id, self.dir, self.layer = uid, dir, layer
        self.version_files = list(version_files)

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
            units[l['id']] = Unit(l['id'], d, True, extra_files.get(l['id'], ()))
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
    def __init__(self, name, bumps, status, body):
        self.name, self.bumps, self.status, self.body = name, bumps, status, body
        self.hash = None
        self.order = (1, name)   # uncommitted: last


def parse_changeset(text, name, units):
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
        if k in RESERVED:
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
    status = fm.get('status', DEFAULT_STATUS)
    if not isinstance(status, str):
        raise Refused(f'{name}: status is one line of text')
    status = '' if status.strip().lower() == 'none' else status.strip().rstrip('.')
    if not body.strip():
        raise Refused(f'{name}: no text under the frontmatter')
    for h in re.findall(r'^#{1,6} .*$', body, re.M):
        if not re.match(r'^### (%s)\s*$' % '|'.join(SECTIONS), h):
            raise Refused(f'{name}: heading {h!r}: only ### {", ### ".join(SECTIONS)}')
    return Changeset(name, bumps, status, body)


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


def load_changesets(root, units):
    out, errs = [], []
    adds = added(root)
    for f in list_changesets(root):
        try:
            cs = parse_changeset(read(root, os.path.join('.changeset', f)), f, units)
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

def plan(root, units, changesets):
    """{id: dict(current, next, bump, changesets=[(name, bump)])} for every unit a
    changeset names, and root, which takes the largest bump of all."""
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
        cur = current(root, unit)
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
    return head + '\n\n' + entry.rstrip('\n') + '\n\n' + rest.lstrip('\n')


def layers_line(units, p):
    lay = [f'{u} {p[u]["next"]}' for u in units if u in p and units[u].layer]
    oth = [f'{u} {p[u]["next"]}' for u in units if u in p and u != ROOT_ID and not units[u].layer]
    if not lay and not oth:
        return None
    return 'Layers: ' + '; '.join(x for x in (', '.join(lay), ', '.join(oth)) if x) + '.'


def apply(root, units, changesets, date, hashes=True):
    """Write the release: entries, version files, the consumed changesets deleted.
    Returns the plan."""
    p = plan(root, units, changesets)
    if not p:
        raise Refused('no pending changesets')
    if not hashes:
        for c in changesets:
            c.hash = None
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
    for c in changesets:
        os.remove(os.path.join(changeset_dir(root), c.name))
    return p


# ---------------------------------------------------------------- check

def check(root, units, base=None, allow_version_change=False):
    errs = []
    cs, perrs = load_changesets(root, units)
    errs += perrs
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
            fm = (f'---\nlayers: [{uid}]\nbump: {bump}\nstatus: {status}\n---\n\n' if uid != ROOT_ID
                  else f'---\nlayers: [root]\nbump: {bump}\nstatus: {status}\n---\n\n')
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


# ---------------------------------------------------------------- add

def slug(text):
    words = re.findall(r'[a-z0-9]+', text.lower())[:6]
    return '-'.join(words) or secrets.token_hex(3)


def add(root, units, bumps, status, section, message, name=None):
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
    parse_changeset(text, fn, units)   # refuse what check would
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


def markdown(p, units, changesets):
    if not p:
        return 'No unreleased changesets: this is the last release plus nothing.\n'
    out = ['| | released | next | bump |', '|---|---|---|---|']
    for uid in units:
        if uid in p:
            e = p[uid]
            out.append(f'| {uid} | {e["current"]} | {e["next"]} | {e["bump"]} |')
    out += ['', '**Unreleased changesets**', '']
    for c in changesets:
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
    a.add_argument('--section', choices=SECTIONS, default=None)
    a.add_argument('--status', help=f'the in-game status line (default "{DEFAULT_STATUS}"; "none" for none)')
    a.add_argument('--name')
    a.add_argument('-m', '--message')
    s = sub.add_parser('status', help='what the next release would bump')
    s.add_argument('--json', action='store_true')
    s.add_argument('--markdown', action='store_true')
    s.add_argument('--next-root', action='store_true',
                   help="print only the root version the next release gets (the current one with none pending)")
    v = sub.add_parser('version', help='consume the changesets into a release')
    v.add_argument('--date', default=datetime.date.today().isoformat())
    v.add_argument('--no-hash', action='store_true', help='cite no commit hashes')
    v.add_argument('--dry-run', action='store_true', help='print the plan, write nothing')
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
                section = ask(f'Section ({", ".join(SECTIONS)})', args.section or
                              ('Fixed' if set(bumps.values()) == {'patch'} else 'Added'))
                status = ask('In-game status ("none" for none)', status or DEFAULT_STATUS)
                msg = ask('What changed (one line; edit the file for more)')
                if not msg:
                    raise Refused('nothing written')
            print(add(root, units, bumps, status, section, msg, args.name))
        elif args.cmd == 'status':
            cs, errs = load_changesets(root, units)
            if errs:
                raise Refused('\n'.join(errs))
            p = plan(root, units, cs)
            if args.next_root:
                print(p[ROOT_ID]['next'] if p else vstr(current(root, units[ROOT_ID])))
            elif args.json:
                print(json.dumps(dict(plan=p, changesets=[c.name for c in cs]), indent=1))
            elif args.markdown:
                sys.stdout.write(markdown(p, units, cs))
            else:
                print_plan(p, units)
        elif args.cmd == 'version':
            cs, errs = load_changesets(root, units)
            if errs:
                raise Refused('\n'.join(errs))
            if args.dry_run:
                print_plan(plan(root, units, cs), units)
                return 0
            p = apply(root, units, cs, args.date, hashes=not args.no_hash)
            print_plan(p, units)
            print(p[ROOT_ID]['next'])
        elif args.cmd == 'check':
            cs, errs = check(root, units, args.base, args.allow_version_change)
            for e in errs:
                print('changeset check:', e, file=sys.stderr)
            if errs:
                return 1
            print(f'{len(cs)} pending changeset(s), {len(units)} units: ok')
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
