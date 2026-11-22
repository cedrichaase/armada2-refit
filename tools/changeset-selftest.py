#!/usr/bin/env python3
"""tools/changeset.py on the host, against fixture layers made up here (a git repository
in a temporary directory) and against this repository's own (check only). Exits 1 on the
first group with a disagreement. CI runs it beside `tools/changeset.py check`."""
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import changeset as C  # noqa: E402

FAIL = []


def check(ok, what):
    print(('ok    ' if ok else 'FAIL  ') + what)
    if not ok:
        FAIL.append(what)


LAYERS = """
schema = 1
[[layer]]
id = 'alpha'
dir = 'alpha'
select = 'on'

[[layer]]
id = 'beta'
dir = 'beta'
select = 'on'

[[layer]]
id = 'gone'
dir = 'retire'
select = 'retired'
"""

CONFIG = """
[[unit]]
id = 'tool'
dir = 'tool'

[[version.tool.version_files]]
file = 'tool/tool.py'
pattern = "(?m)^VERSION = '(?P<v>[^']+)'"
"""


GAME_ONE = "id = 'one'\nlabel = 'Game One'\noffered = true\ndefault = true\n"
GAME_TWO = ("id = 'two'\nlabel = 'Game Two'\noffered = false\ndefault = false\n"
            "[changesets]\nquiet = ['Secret Mod']\n")


def log(title, v):
    return f'# Changelog — {title}\n\nIntro.\n\n## {v} — 2026-01-01\n\n### Added\n- First.\n\nConfirmed in game 2026-01-02.\n'


def git(root, *a):
    subprocess.run(['git', '-C', root, *a], check=True, stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL)


def fixture():
    root = tempfile.mkdtemp(prefix='changeset-selftest-')
    files = {
        'layers.toml': LAYERS,
        'games/one.toml': GAME_ONE,
        'games/two.toml': GAME_TWO,
        '.changeset/config.toml': CONFIG,
        'CHANGELOG.md': log('project', '3.4.5'),
        'alpha/CHANGELOG.md': log('alpha', '1.2.3'),
        'alpha/Cargo.toml': '[package]\nname = "alpha"\nversion = "1.2.3"\nedition = "2021"\n\n[dependencies]\nx = { version = "1.2.3" }\n',
        'alpha/Cargo.lock': 'version = 3\n\n[[package]]\nname = "alpha"\nversion = "1.2.3"\n\n[[package]]\nname = "x"\nversion = "1.2.3"\n',
        'beta/CHANGELOG.md': log('beta', '0.4.0'),
        'tool/CHANGELOG.md': log('tool', '2.0.0'),
        'tool/tool.py': "VERSION = '2.0.0'\n",
    }
    for rel, text in files.items():
        os.makedirs(os.path.dirname(os.path.join(root, rel)) or root, exist_ok=True)
        with open(os.path.join(root, rel), 'w') as fh:
            fh.write(text)
    git(root, 'init', '-q', '-b', 'main')
    git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'add', '-A')
    git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-qm', 'base')
    return root


def put(root, name, text, commit=True):
    with open(os.path.join(root, '.changeset', name), 'w') as fh:
        fh.write(text)
    if commit:
        git(root, 'add', '-A')
        git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-qm', name)


def rd(root, rel):
    with open(os.path.join(root, rel)) as fh:
        return fh.read()


def refused(text, units):
    try:
        C.parse_changeset(text, 'x.md', units)
    except C.Refused as e:
        return str(e)
    return None


def main():
    root = fixture()
    units = C.load_units(root)
    check(list(units) == ['alpha', 'beta', 'tool', 'root'],
          f'units: layers (retired left out), config units, root: {list(units)}')

    # ------------------------------------------------ parsing
    cs = C.parse_changeset('---\nlayers: [alpha, beta]\nbump: minor\ngames: [one]\n---\n\n- x\n', 'a.md', units)
    check(cs.bumps == {'alpha': 'minor', 'beta': 'minor'} and cs.status == C.DEFAULT_STATUS,
          'layers + bump applies to each; status defaults to "not yet seen"')
    cs = C.parse_changeset('---\n"alpha": major\nbeta: patch\nstatus: none\ngames: [one]\n---\n\n- x\n', 'a.md', units)
    check(cs.bumps == {'alpha': 'major', 'beta': 'patch'} and cs.status == '',
          "pnpm's per-layer form, quoted keys; status: none")
    cs = C.parse_changeset('---\nlayers: [alpha, beta]\nbump: patch\nbeta: major\ngames: [one]\n---\n\n- x\n', 'a.md', units)
    check(cs.bumps == {'alpha': 'patch', 'beta': 'major'}, 'a per-layer bump overrides bump:')
    for text, want in [
        ('---\nlayers: [alpha]\ngames: [one]\n---\n\n- x\n', 'without a bump'),
        ('---\nlayers: [alpha]\nbump: huge\ngames: [one]\n---\n\n- x\n', 'is not major'),
        ('---\nalpha: big\ngames: [one]\n---\n\n- x\n', 'is not major'),
        ('---\nbump: minor\ngames: [one]\n---\n\n- x\n', 'bump without layers'),
        ('---\nstatus: x\ngames: [one]\n---\n\n- x\n', 'no bump'),
        ('---\ngames: [one]\n---\n\n- x\n', 'no bump'),
        ('---\nlayers: [gone]\nbump: minor\ngames: [one]\n---\n\n- x\n', 'no such layer'),
        ('---\nlayers: [alpha]\nbump: minor\ngames: [one]\n---\n\n', 'no text'),
        ('---\nlayers: [alpha]\nbump: minor\ngames: [one]\n---\n\n## 1.0.0\n- x\n', 'heading'),
        ('---\nlayers: [alpha]\nbump: minor\ngames: [one]\n---\n\n### Improved\n- x\n', 'heading'),
        ('layers: [alpha]\n', 'no frontmatter'),
        ('---\nlayers: []\nbump: minor\ngames: [one]\n---\n\n- x\n', 'layers is empty'),
    ]:
        e = refused(text, units)
        check(e is not None and want in e, f'refused ({want}): {e}')

    # ------------------------------------------------ plan and status
    put(root, 'one.md', '---\nlayers: [alpha]\nbump: patch\ngames: [one]\n---\n\n### Fixed\n- alpha fix one.\n')
    put(root, 'two.md', '---\nlayers: [alpha, beta]\nbump: minor\ngames: [one]\n---\n\nShared preamble.\n\n### Added\n- a new key, `Key=`\n  wrapped line.\n')
    put(root, 'three.md', '---\nroot: patch\nstatus: none\ngames: [one]\n---\n\n- a repo-wide convention\n')
    put(root, 'four.md', '---\ntool: major\nstatus: Confirmed in game 2026-02-02\ngames: [one]\n---\n\n### Removed\n- the old flag\n')
    cs, errs = C.load_changesets(root, units)
    check(not errs and [c.name for c in cs] == ['one.md', 'two.md', 'three.md', 'four.md'],
          f'changesets in the order they were committed: {[c.name for c in cs]} {errs}')
    p = C.plan(root, units, cs)
    check(p['alpha']['current'] == '1.2.3' and p['alpha']['next'] == '1.3.0',
          f'alpha: the largest of patch and minor: {p["alpha"]}')
    check(p['beta']['next'] == '0.5.0', f'beta 0.4.0 minor -> 0.5.0: {p["beta"]["next"]}')
    check(p['tool']['next'] == '3.0.0', f'tool major: {p["tool"]["next"]}')
    check(p['root']['next'] == '4.0.0' and p['root']['bump'] == 'major',
          f'root: the largest bump of all: {p["root"]}')
    md = C.markdown(p, units, cs)
    check('| alpha | 1.2.3 | 1.3.0 | minor |' in md and '`two.md`' in md, 'status --markdown')

    # ------------------------------------------------ version
    p = C.apply(root, units, cs, '2026-03-03')
    a = rd(root, 'alpha/CHANGELOG.md')
    check(a.index('## 1.3.0 — 2026-03-03') < a.index('## 1.2.3') and a.startswith('# Changelog — alpha\n\nIntro.\n\n## 1.3.0'),
          'alpha: the entry goes above the newest, below the header')
    entry = a[a.index('## 1.3.0'):a.index('## 1.2.3')]
    check('Shared preamble.' in entry and entry.index('### Added') < entry.index('### Fixed'),
          'alpha: preamble, then sections in Added/Changed/Fixed/Removed order')
    check('wrapped line. `' in entry and 'alpha fix one. `' in entry,
          'alpha: each item cites the commit that added its changeset')
    check(entry.rstrip().endswith('Installed, not yet seen in game.'),
          'alpha: one status line when every changeset agrees')
    r = rd(root, 'CHANGELOG.md')
    rentry = r[r.index('## 4.0.0'):r.index('## 3.4.5')]
    check('Layers: alpha 1.3.0, beta 0.5.0; tool 3.0.0.' in rentry, 'root: the Layers line')
    check('- alpha, beta: a new key' in rentry and '- alpha: alpha fix one.' in rentry
          and '- a repo-wide convention' in rentry and '- tool: the old flag' in rentry,
          'root: items led by the layers they touch; a root-only item is not')
    check('Shared preamble' not in rentry, 'root: no preambles')
    check('Confirmed in game 2026-02-02.' in rentry and rentry.count('Installed, not yet seen in game.') == 2,
          'root: mixed statuses ride on each item')
    check(rd(root, 'alpha/Cargo.toml').count('version = "1.3.0"') == 1
          and 'x = { version = "1.2.3" }' in rd(root, 'alpha/Cargo.toml'),
          'Cargo.toml: the package version only')
    lock = rd(root, 'alpha/Cargo.lock')
    check('name = "alpha"\nversion = "1.3.0"' in lock and 'name = "x"\nversion = "1.2.3"' in lock,
          'Cargo.lock: the package entry only')
    check(rd(root, 'tool/tool.py') == "VERSION = '3.0.0'\n", 'config version_files')
    check(C.list_changesets(root) == [], 'the consumed changesets are deleted')
    check(not C.check(root, units)[1], 'check passes after version')
    check(C.plan(root, units, []) == {}, 'nothing pending: an empty plan')
    before = rd_rev(root, 'HEAD')
    git(root, 'add', '-A')
    git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-qm',
        'Release 4.0.0\n\nRelease-Version: 4.0.0')
    check(C.released(root, f'{before}..HEAD') == '4.0.0', 'released: the release commit is found')
    check(C.released(root, 'HEAD..HEAD') is None, 'released: nothing in an empty range')

    # ------------------------------------------------ check
    with open(os.path.join(root, 'tool/tool.py'), 'w') as fh:
        fh.write("VERSION = '9.9.9'\n")
    e = C.check(root, units)[1]
    check(any('tool/tool.py' in x and '9.9.9' in x for x in e), f'check: a version file out of step: {e}')
    with open(os.path.join(root, 'tool/tool.py'), 'w') as fh:
        fh.write("VERSION = '3.0.0'\n")
    put(root, 'bad.md', '---\nlayers: [nope]\nbump: minor\ngames: [one]\n---\n\n- x\n', commit=False)
    check(any('no such layer' in x for x in C.check(root, units)[1]), 'check: a bad changeset')
    os.remove(os.path.join(root, '.changeset', 'bad.md'))

    # ------------------------------------------------ convert (an in-flight branch)
    git(root, 'checkout', '-qb', 'feature')
    for rel, v, body in [('beta/CHANGELOG.md', '0.5.1', '### Fixed\n- beta fix. Installed, not yet seen in game.\n'),
                         ('beta/CHANGELOG.md', '0.6.0', '### Added\n- beta thing.\n\nConfirmed in game 2026-04-04.\n'),
                         ('alpha/CHANGELOG.md', '2.0.0', '### Changed\n- alpha breaks.\n\nConfirmed in game 2026-04-04.\n'),
                         ('CHANGELOG.md', '4.1.0', 'Layers: beta 0.6.0, alpha 2.0.0.\n\n### Added\n- summary.\n')]:
        t = rd(root, rel)
        i = t.index('## ')
        with open(os.path.join(root, rel), 'w') as fh:
            fh.write(t[:i] + f'## {v} — 2026-04-04\n\n{body}\n' + t[i:])
    for f, old, new in [('alpha/Cargo.toml', 'version = "1.3.0"', 'version = "2.0.0"'),
                        ('alpha/Cargo.lock', 'name = "alpha"\nversion = "1.3.0"', 'name = "alpha"\nversion = "2.0.0"')]:
        t = rd(root, f)
        with open(os.path.join(root, f), 'w') as fh:
            fh.write(t.replace(old, new))
    git(root, 'add', '-A')
    git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-qm', 'hand entries')
    before = {f: rd(root, f) for f in ('beta/CHANGELOG.md', 'alpha/CHANGELOG.md', 'CHANGELOG.md')}
    try:
        C.released(root, 'main..HEAD')
        check(False, 'released: a version changed without the trailer fails loudly')
    except C.Untrailed as e:
        check('squash' in str(e) and 'release=true' in str(e),
              'released: a version changed without the trailer (a squash, a hand edit) fails loudly')
    e = C.check(root, units, base='main')[1]
    check(any('by hand' in x for x in e), 'check --base: a hand-numbered version is refused')
    check(not C.check(root, units, base='main', allow_version_change=True)[1],
          'check --base --allow-version-change: the release PR passes')
    made, notes = C.convert(root, units, 'main')
    names = {m[0]: m for m in made}
    check(set(names) == {'converted-alpha.md', 'converted-beta.md'} and notes,
          f'convert: one changeset per layer, the root entry dropped with a note: {list(names)} {notes}')
    check(names['converted-beta.md'][5] == 'minor' and names['converted-alpha.md'][5] == 'major',
          'convert: the bump from main\'s version to the branch\'s newest')
    bt = rd(root, '.changeset/converted-beta.md')
    check('status: Installed, not yet seen in game' in bt and '- beta fix.' in bt
          and '- beta thing.' in bt and 'Confirmed' not in bt and 'not yet seen in game.' not in bt.split('---')[2],
          'convert: statuses lifted into the frontmatter, "not yet seen" wins')
    at = rd(root, '.changeset/converted-alpha.md')
    check('status: Confirmed in game 2026-04-04' in at, 'convert: all confirmed stays confirmed')
    check(rd(root, 'beta/CHANGELOG.md') == rd_base(root, 'beta/CHANGELOG.md')
          and rd(root, 'CHANGELOG.md') == rd_base(root, 'CHANGELOG.md'),
          'convert: the changelogs are main\'s again')
    check('version = "1.3.0"' in rd(root, 'alpha/Cargo.toml')
          and 'name = "alpha"\nversion = "1.3.0"' in rd(root, 'alpha/Cargo.lock'),
          'convert: the version files are main\'s again')
    units = C.load_units(root)
    cs, errs = C.load_changesets(root, units)
    p = C.plan(root, units, cs)
    check(not errs and p['beta']['next'] == '0.6.0' and p['alpha']['next'] == '2.0.0'
          and p['root']['next'] == '5.0.0', f'convert: the release gives the same versions back: {p}')
    del before

    # ------------------------------------------------ add
    path = C.add(root, units, {'alpha': 'patch', 'beta': 'patch'}, None, 'Fixed', 'Fix the thing')
    t = rd(root, path)
    check(t == '---\nlayers: [alpha, beta]\nbump: patch\ngames: [one]\n---\n\n### Fixed\n- Fix the thing\n'
          and path == os.path.join('.changeset', 'fix-the-thing.md'), f'add: {path} {t!r}')
    path = C.add(root, units, {'alpha': 'minor', 'tool': 'patch'}, 'none', 'Added', 'Two bumps')
    t = rd(root, path)
    check('alpha: minor\ntool: patch\ngames: [one]\nstatus: none\n' in t, 'add: the per-layer form for different bumps')
    try:
        C.add(root, units, {'alpha': 'giant'}, None, 'Fixed', 'x')
        check(False, 'add refuses an invalid bump')
    except C.Refused:
        check(True, 'add refuses an invalid bump')

    # ------------------------------------------------ version --commit
    r2 = fixture()
    put(r2, 'x.md', '---\nlayers: [beta]\nbump: patch\ngames: [one]\n---\n\n- x\n')
    head = rd_rev(r2, 'HEAD')
    env = dict(os.environ, GIT_AUTHOR_NAME='t', GIT_AUTHOR_EMAIL='t@t',
               GIT_COMMITTER_NAME='t', GIT_COMMITTER_EMAIL='t@t')
    out = subprocess.run([sys.executable, os.path.join(HERE, 'changeset.py'), '--root', r2,
                          'version', '--commit', '--date', '2026-05-05'],
                         stdout=subprocess.PIPE, text=True, env=env)
    msg = subprocess.run(['git', '-C', r2, 'log', '-1', '--format=%B'], stdout=subprocess.PIPE,
                         text=True).stdout
    check(out.returncode == 0 and msg.startswith('Release 3.4.6\n') and 'Release-Version: 3.4.6' in msg
          and C.released(r2, f'{head}..HEAD') == '3.4.6'
          and not subprocess.run(['git', '-C', r2, 'status', '--porcelain'], stdout=subprocess.PIPE,
                                 text=True).stdout.strip(),
          f'version --commit: one commit "Release 3.4.6" with its trailer: {msg!r}')
    nr = subprocess.run([sys.executable, os.path.join(HERE, 'changeset.py'), '--root', r2,
                         'status', '--next-root'], stdout=subprocess.PIPE, text=True).stdout.strip()
    check(nr == '3.4.7', f'status --next-root with nothing pending: the current plus a patch ({nr})')
    subprocess.run(['rm', '-rf', r2])

    # ------------------------------------------------ games: one version track per game
    games_test()

    # ------------------------------------------------ this repository
    repo = os.path.dirname(HERE)
    ru = C.load_units(repo)
    check({'display', 'render3d', 'ui', 'qol', 'gui-installer', 'testbench', 'root'} <= set(ru)
          and 'hud' not in ru, f'this repository\'s units: {", ".join(ru)}')
    e = C.check(repo, ru)[1]
    check(not e, f'this repository passes check: {e}')

    subprocess.run(['rm', '-rf', root])
    if FAIL:
        print(f'\n{len(FAIL)} failed')
        return 1
    print('\nall ok')
    return 0


def run(root, *args):
    r = subprocess.run([sys.executable, os.path.join(HERE, 'changeset.py'), '--root', root, *args],
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    return r.returncode, r.stdout, r.stderr


def games_test():
    root = fixture()
    units = C.load_units(root)
    games = C.load_games(root)
    check(list(games) == ['one', 'two'] and games['one'].default and not games['two'].offered
          and games['two'].changelog == os.path.join('games', 'two', 'CHANGELOG.md')
          and games['one'].changelog is None, f'games: the default first: {list(games)}')

    # parsing
    cs = C.parse_changeset('---\nlayers: [alpha]\nbump: minor\ngames: [two, one]\n'
                           'status.two: Confirmed in game 2026-06-06\n---\n\n- x\n', 'a.md', units, games)
    check(cs.games == ['one', 'two'] and cs.status_for('one') == C.DEFAULT_STATUS
          and cs.status_for('two') == 'Confirmed in game 2026-06-06',
          'games in the registry\'s order; status.<game> overrides status for that game only')
    for text, want in [
        ('---\nlayers: [alpha]\nbump: minor\n---\n\n- x\n', 'no games'),
        ('---\nlayers: [alpha]\nbump: minor\ngames: []\n---\n\n- x\n', 'games is empty'),
        ('---\nlayers: [alpha]\nbump: minor\ngames: [three]\n---\n\n- x\n', 'no such game'),
        ('---\nlayers: [alpha]\nbump: minor\ngames: [one, one]\n---\n\n- x\n', 'twice'),
        ('---\nlayers: [alpha]\nbump: minor\ngames: [one]\nstatus.two: x\n---\n\n- x\n', 'not one of its games'),
    ]:
        try:
            C.parse_changeset(text, 'x.md', units, games)
            e = None
        except C.Refused as x:
            e = str(x)
        check(e is not None and want in e, f'refused ({want}): {e}')

    # a layer that is not a game's takes no bump on that game's track
    units['beta'].games = ['one']
    cs = C.parse_changeset('---\nlayers: [alpha, beta]\nbump: minor\ngames: [one, two]\n---\n\n- x\n',
                           'a.md', units, games)
    check(cs.on('two').bumps == {'alpha': 'minor'} and cs.on('one').bumps == {'alpha': 'minor', 'beta': 'minor'},
          'a layer off a game (layers.toml games) is left out of its track')
    try:
        C.parse_changeset('---\nlayers: [beta]\nbump: minor\ngames: [two]\n---\n\n- x\n', 'a.md', units, games)
        check(False, 'refused: a changeset naming no layer of its game')
    except C.Refused as x:
        check('names no unit of two' in str(x), f'refused: a changeset naming no layer of its game: {x}')
    units['beta'].games = None

    # check: a published track never names a game that is not offered
    put(root, 'leak.md', '---\nlayers: [alpha]\nbump: patch\ngames: [one, two]\n---\n\n- for the Secret Mod.\n', commit=False)
    e = C.check(root, units)[1]
    check(any('leak.md' in x and 'Secret Mod' in x for x in e), f'check: a quiet word on a published track: {e}')
    put(root, 'leak.md', '---\nlayers: [alpha]\nbump: patch\ngames: [two]\n---\n\n- for the Secret Mod, in Game Two.\n', commit=False)
    check(not C.check(root, units)[1], 'check: a quiet word on the quiet game\'s own track is fine')
    os.remove(os.path.join(root, '.changeset', 'leak.md'))
    # check: a file players receive never names it either (publish/README.md)
    put(root, '../PLAYING.md', 'ok\nSee the Secret Mod.\n', commit=False)
    e = C.check(root, units)[1]
    check(any('PLAYING.md:2' in x and 'Secret Mod' in x for x in e), f'check: a quiet word in PLAYING.md: {e}')
    os.remove(os.path.join(root, 'PLAYING.md'))
    try:
        C.add(root, units, {'alpha': 'patch'}, None, 'Fixed', 'two: fix', games=['one'])
        check(False, 'add refuses a quiet game\'s id on a published track')
    except C.Refused as x:
        check('two' in str(x), 'add refuses a quiet game\'s id on a published track')
    path = C.add(root, units, {'alpha': 'patch'}, None, 'Fixed', 'Only two', games=['two'], name='only-two')
    check('games: [two]\n' in rd(root, path), 'add --games')
    os.remove(os.path.join(root, path))

    # plan and version per game
    put(root, 'shared.md', '---\nlayers: [alpha, beta]\nbump: minor\ngames: [one, two]\n'
        'status.two: Confirmed in game 2026-06-06\n---\n\n### Added\n- shared thing.\n')
    put(root, 'one-only.md', '---\nlayers: [beta]\nbump: patch\ngames: [one]\n---\n\n### Fixed\n- one fix.\n')
    put(root, 'two-only.md', '---\nlayers: [alpha]\nbump: major\ngames: [two]\n---\n\n### Changed\n- two breaks.\n')
    put(root, 'two-beta.md', '---\nlayers: [beta]\nbump: patch\ngames: [two]\n---\n\n### Fixed\n- two beta fix.\n')
    cs, errs = C.load_changesets(root, units, games)
    p1 = C.plan(root, units, cs, games['one'])
    p2 = C.plan(root, units, cs, games['two'])
    check(not errs and p1['alpha']['next'] == '1.3.0' and p1['beta']['next'] == '0.5.0'
          and p1['root']['next'] == '3.5.0' and [n for n, _ in p1['root']['changesets']] == ['shared.md', 'one-only.md'],
          f'plan one: its changesets only, on the version files\' track: {p1}')
    check(p2['alpha']['current'] == '0.0.0' and p2['alpha']['next'] == '1.0.0'
          and p2['beta']['next'] == '0.1.0' and p2['root']['next'] == '1.0.0',
          f'plan two: from 0.0.0 on a new track: {p2}')
    rc, out, _ = run(root, 'status')
    check(rc == 0 and '== one' in out and '== two (games/two/CHANGELOG.md)' in out,
          'status: every game\'s track')
    rc, out, _ = run(root, 'status', '--json')
    import json
    j = json.loads(out)
    check(j['game'] == 'one' and j['changesets'] == ['shared.md', 'one-only.md'],
          f'status --json: the default game\'s track: {j["changesets"]}')
    rc, out, _ = run(root, 'status', '--markdown')
    check('two-only' not in out and 'two breaks' not in out and '`one-only.md`' in out,
          'status --markdown: the default game\'s changesets only')
    rc, out, _ = run(root, 'status', '--next-root', '--game', 'two')
    check(out.strip() == '1.0.0', f'status --next-root --game two: {out.strip()}')

    rc, _, err = run(root, 'version', '--game', 'two', '--commit')
    check(rc == 1 and 'default game only' in err, f'version --game two --commit is refused: {err.strip()}')
    rc, out, err = run(root, 'version', '--game', 'two', '--date', '2026-07-07')
    t = rd(root, 'games/two/CHANGELOG.md')
    check(rc == 0 and t.startswith('# Changelog — Game Two\n') and '## 1.0.0 — 2026-07-07' in t
          and 'Layers: alpha 1.0.0, beta 0.1.0.' in t, f'version --game two: the game\'s changelog: {err}')
    check('- alpha, beta: shared thing' in t and '- alpha: two breaks' in t and '- beta: two beta fix' in t,
          'version --game two: items led by their layers')
    check('two breaks' in t and 'shared thing' in t and 'one fix' not in t
          and 'Confirmed in game 2026-06-06' in t, 'version --game two: its items, its status')
    check(sorted(C.list_changesets(root)) == ['one-only.md', 'shared.md'],
          f'version --game two: its own changesets deleted, shared ones kept: {C.list_changesets(root)}')
    sh = rd(root, '.changeset/shared.md')
    check('games: [one]\n' in sh and 'status.two' not in sh, f'shared: two taken off games: {sh!r}')
    check(rd(root, 'alpha/CHANGELOG.md') == rd_base(root, 'alpha/CHANGELOG.md')
          and "VERSION = '2.0.0'" in rd(root, 'tool/tool.py')
          and rd(root, 'CHANGELOG.md') == rd_base(root, 'CHANGELOG.md'),
          'version --game two: no version file and no published changelog touched')
    check(not C.check(root, units)[1], 'check passes after version --game two')
    git(root, 'add', '-A')
    git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-qm', 'two 1.0.0')

    # a second release on the track starts from the first's versions
    put(root, 'two-again.md', '---\nlayers: [beta]\nbump: minor\ngames: [two]\n---\n\n- two again.\n')
    cs, _ = C.load_changesets(root, units, games)
    p2 = C.plan(root, units, cs, games['two'])
    check(p2['beta']['current'] == '0.1.0' and p2['beta']['next'] == '0.2.0' and p2['root']['next'] == '1.1.0',
          f'track versions read back from games/two/CHANGELOG.md: {p2}')
    C.apply(root, units, cs, '2026-08-08', game=games['two'])
    t = rd(root, 'games/two/CHANGELOG.md')
    check(t.index('## 1.1.0') < t.index('## 1.0.0') and 'Layers: alpha 1.0.0, beta 0.2.0.' in t,
          'the Layers line gives every layer\'s version on the track')

    # the default game's release: as before, its changesets only
    rc, out, err = run(root, 'version', '--date', '2026-09-09')
    r = rd(root, 'CHANGELOG.md')
    check(rc == 0 and '## 3.5.0 — 2026-09-09' in r and 'shared thing' in r and 'two' not in r.split('## 3.4.5')[0],
          f'version (default game): the root entry of its changesets: {err}')
    check(C.list_changesets(root) == [], 'every changeset consumed by both tracks is gone')
    subprocess.run(['rm', '-rf', root])


def rd_rev(root, rev):
    return subprocess.run(['git', '-C', root, 'rev-parse', rev], stdout=subprocess.PIPE,
                          text=True, check=True).stdout.strip()


def rd_base(root, rel):
    return subprocess.run(['git', '-C', root, 'show', f'main:{rel}'], stdout=subprocess.PIPE,
                          text=True, check=True).stdout


if __name__ == '__main__':
    sys.exit(main())
