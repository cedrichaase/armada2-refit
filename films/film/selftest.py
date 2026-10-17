#!/usr/bin/env python3
"""Pins when a take counts as current (films/README.md, "When a take is current"), on a
throw-away git repository laid out like this one: a take is current until a file it
depends on changes, committed or not; docs, the boards and the stock variant's unrelated
layers do not count; a take filmed from uncommitted changes is current while the tree
still has them; takes from before films 1.1.0 are read as they were written. Then the
board's own rules, on throw-away boards and footage: a board split over files with
`Include:`, a shot's `Tier:`, the take a cut picks, `adopt`, and the reference
catalogue. Starts no game. Exit 0 when all of it holds.

    python3 films/film/selftest.py
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from film import cli, code, storyboard as sb  # noqa: E402

BOARD = """# Code test
Film: codetest

## Test

### t01 · A scene in two variants
Scene: planet
Variants: stock, refit

### d01 · The installer
Kind: desktop
Run: python3 {repo}/gui-installer/installer.py
"""

failed = []


def check(ok, what):
    print(('ok    ' if ok else 'FAIL  ') + what)
    if not ok:
        failed.append(what)


def write(repo, path, text):
    p = os.path.join(repo, path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        f.write(text)


def git(repo, *args):
    subprocess.run(['git', '-C', repo, *args], check=True, capture_output=True)


def commit(repo, msg):
    git(repo, 'add', '-A')
    git(repo, '-c', 'user.name=selftest', '-c', 'user.email=selftest@invalid', 'commit', '-q', '-m', msg)
    return code.head(repo)


SPLIT = """# Split test
Film: split

The film's prose.

Include: parts/intro.md

## Two
Music: bed.wav

Include: parts/two-shots.md

### s05 · Back in the board
Scene: planet
Length: 2

```action
wait 3
```
"""

INTRO = """# Intro, part of films/split

This preamble is for the reader of this file and is not read.
Status: final

## One

### s01 · First
Scene: planet
Length: 2
Tier: 1080

```action
wait 3
```

### s02 · Second
Scene: planet
Variants: stock, refit
Layout: dissolve
Length: 2
Tier: final

```action
wait 3
```
"""

TWO = """### s03 · Third
Scene: planet
Length: 2

```action
wait 3
```

Include: s04.md
"""

S04 = """### s04 · Fourth
Kind: footage
From: ref:fleet01
Length: 2
"""

CATALOGUE = """# Reference catalogue (test)

### fleet01 · A fleet at warp
Take: scout m01 refit final
Mission: a2_borg01, the intro
In: 12.5
Out: 20
Audio: dialogue over the whole of it
Quality: good

A fleet drops out of warp.

### ex01 · An example
File: films/reference/example.mkv
Quality: example

### bad01 · No source
Quality: poor
In: 5
Out: 4
"""


def write_take(stem, film, shot, variant, tier, spec_hash=None):
    os.makedirs(os.path.dirname(stem), exist_ok=True)
    for ext in ('.mkv', '.wav', '.events.json'):
        with open(stem + ext, 'w') as f:
            f.write(f'{film.name} {shot.id} {variant} {tier}\n')
    json.dump(dict(hash=spec_hash or film.spec_hash(shot, variant, tier), spec=film.spec(shot, variant, tier),
                   commit=None, res=film.res(tier, shot), tier=tier), open(stem + '.spec.json', 'w'))


def raises(fn, what, text):
    try:
        fn()
    except sb.BoardError as e:
        check(text in str(e), f'{what} ({e})')
        return
    check(False, f'{what} (no error)')


def adopt(src, ids, dst, **kw):
    a = argparse.Namespace(board=src, shots=ids + [dst], as_id=kw.get('as_id'), tier=None,
                           force=kw.get('force', False), dry_run=False)
    return cli.adopt(a)


def boards():
    with tempfile.TemporaryDirectory() as root:
        data = os.path.join(root, 'data')
        sb.a2env.DATA = data           # footage here, not in $A2_DATA
        films = os.path.join(root, 'films')
        board = os.path.join(films, 'split', 'storyboard.md')
        write(root, 'films/split/storyboard.md', SPLIT)
        write(root, 'films/split/parts/intro.md', INTRO)
        write(root, 'films/split/parts/two-shots.md', TWO)
        write(root, 'films/split/parts/s04.md', S04)
        write(root, 'films/reference/catalogue.md', CATALOGUE)
        write(root, 'films/reference/README.md', '### nope · the README is not a catalogue\n')

        # -------------------------------------------------- a board over several files
        film = sb.parse(board)
        check([s.id for s in film.shots] == ['s01', 's02', 's03', 's04', 's05'],
              'included shots stand where their Include: does, in order, nested too')
        check([sec.title for sec in film.sections] == ['One', 'Two'] and
              [s.id for s in film.sections[1].shots] == ['s03', 's04', 's05'],
              'an Include: under a section adds its shots to that section')
        check(film.files == ['parts/intro.md', 'parts/two-shots.md', 'parts/s04.md'], 'the board lists its files')
        check(film.shot('s01').file == 'parts/intro.md' and film.shot('s05').file == 'storyboard.md',
              'a shot knows its file')
        check('preamble' not in '\n'.join(film.prose + film.sections[0].prose) and 'Status' not in film.fields,
              "an included file's preamble is not read")
        check(film.sections[1].fields.get('Music') == 'bed.wav', 'section fields stand before an Include:')

        # moving a shot from one file to another keeps its spec, and so its takes
        h = film.spec_hash(film.shot('s03'), 'refit', 'probe')
        write(root, 'films/split/parts/two-shots.md', 'Include: s04.md\n')
        write(root, 'films/split/parts/s04.md', S04 + '\n' + TWO.replace('Include: s04.md', ''))
        moved = sb.parse(board)
        check(moved.shot('s03').file == 'parts/s04.md' and moved.spec_hash(moved.shot('s03'), 'refit', 'probe') == h,
              'a shot moved to another file of the film keeps its spec')
        write(root, 'films/split/parts/two-shots.md', TWO)
        write(root, 'films/split/parts/s04.md', S04)

        write(root, 'films/split/parts/dup.md', '## Dup\n### s01 · Again\n')
        write(root, 'films/split/storyboard.md', SPLIT + '\nInclude: parts/dup.md\n')
        raises(lambda: sb.parse(board), 'a shot id used in two files is an error', 'used twice')
        write(root, 'films/split/parts/loop.md', '## Loop\nInclude: ../storyboard.md\n')
        write(root, 'films/split/storyboard.md', SPLIT + '\nInclude: parts/loop.md\n')
        raises(lambda: sb.parse(board), 'a file that includes itself is an error', 'included in itself')
        write(root, 'films/other/x.md', '## X\n')
        write(root, 'films/split/storyboard.md', SPLIT + '\nInclude: ../other/x.md\n')
        raises(lambda: sb.parse(board), "a file outside the film's folder is an error", "not in the film's folder")
        write(root, 'films/split/storyboard.md', SPLIT + '\nInclude: parts/missing.md\n')
        raises(lambda: sb.parse(board), 'a missing file is an error', 'no such file')
        write(root, 'films/split/storyboard.md', SPLIT + '\n```\nInclude: parts/missing.md\n```\n')
        check(sb.parse(board).shot('s05') is not None, 'an Include: in a fenced block is text')
        write(root, 'films/split/storyboard.md', SPLIT)

        # -------------------------------------------------- a shot's tier
        s01, s02, s03 = film.shot('s01'), film.shot('s02'), film.shot('s03')
        check(film.res('probe', s01) == '1920x1080' and film.res('final', s01) == '3840x2160',
              'Tier: 1080 probes at 1920x1080; its final take is at Final')
        check(film.res('probe', s03) == '1280x720' and film.spec(s03, 'refit', 'probe')['res'] == '1280x720',
              'a shot without Tier: probes at Probe')
        check(film.shot_tier(s02, 'probe') == 'final' and film.shot_tier(s01, 'probe') == 'probe'
              and film.shot_tier(s02, 'final') == 'final', 'Tier: final films the final take in a probe run')
        check(sb.tier_res('4k') == '3840x2160' and sb.tier_res('1440p') == '2560x1440'
              and sb.tier_res('1080', '2560x1080') == '2560x1080' and sb.tier_res('1600x900') == '1600x900',
              'tiers by height keep the probe aspect')
        raises(lambda: sb.tier_res('huge'), 'an unknown tier is an error', 'Tier huge')
        h3 = film.spec_hash(s03, 'refit', 'probe')
        for t in ('2160', '4k', '3840x2160', '4K'):
            s03.fields['Tier'] = t
            check(s03.tier == 'final' and film.shot_tier(s03, 'probe') == 'final', f'Tier: {t} equal to Final is Tier: final')
        s03.fields['Tier'] = '1440'
        check(s03.tier == '1440' and film.shot_tier(s03, 'probe') == 'probe', 'a Tier: below Final stays a larger probe')
        film.fields['Final'] = '2560x1440'
        check(s03.tier == 'final', 'Tier: 1440 is final when Final is 2560x1440')
        del film.fields['Final']
        del s03.fields['Tier']
        check(film.spec_hash(s03, 'refit', 'probe') == h3, 'a shot without Tier: keeps its spec hash')
        h1 = film.spec_hash(s01, 'refit', 'probe')
        s01.fields['Tier'] = '720'
        check(film.spec_hash(s01, 'refit', 'probe') != h1, "a changed Tier: makes the shot's probe stale")
        s01.fields['Tier'] = '1080'
        s03.fields['Tier'] = 'huge'
        check(any(s is s03 and 'Tier huge' in m for s, m in sb.check(film)), 'check reports a bad Tier:')
        del s03.fields['Tier']

        # -------------------------------------------------- the take a cut picks
        def best(shot, v, tier='probe'):
            return cli.best_take(film, shot, v, tier, ignore_code=True)[1]
        check(best(s03, 'refit') is None, 'no take: none')
        write_take(film.take(s03, 'refit', 'probe'), film, s03, 'refit', 'probe')
        check(best(s03, 'refit') == 'probe' and best(s03, 'refit', 'final') is None,
              'a probe cut takes the probe; a final cut does not')
        write_take(film.take(s03, 'refit', 'final'), film, s03, 'refit', 'final')
        check(best(s03, 'refit') == 'final', 'a current final take beats a current probe')
        write_take(film.take(s03, 'refit', 'final'), film, s03, 'refit', 'final', spec_hash='0' * 16)
        check(best(s03, 'refit') == 'probe', '... but not when its spec changed')
        write_take(film.take(s02, 'stock', 'final'), film, s02, 'stock', 'final')
        check(best(s02, 'stock') == 'final', 'a Tier: final shot is cut from its final take')

        # -------------------------------------------------- adopt
        write(root, 'films/scout/storyboard.md', '# Scout\nFilm: scout\n\n' + INTRO.split('## One', 1)[1]
              .replace('Tier: 1080', 'Tier: 1080\nStatus: probed').replace('### s02', '### x02'))
        scout = sb.parse(os.path.join(films, 'scout'))
        sx1 = scout.shot('s01')
        write_take(scout.take(sx1, 'refit', 'probe'), scout, sx1, 'refit', 'probe')
        check(not os.path.exists(film.take(s01, 'refit', 'probe') + '.mkv'), 'the target has no take of s01')
        check(cli.take_state(film, s01, 'refit', 'probe', True) == '-', '... so it is not filmed')
        rc = adopt(os.path.join(films, 'scout'), ['s01'], os.path.join(films, 'split'))
        check(rc == 0 and cli.take_state(film, s01, 'refit', 'probe', True) == 'ok',
              'adopt takes over a take of the same spec from another film, current')
        sp = json.load(open(film.take(s01, 'refit', 'probe') + '.spec.json'))
        check(sp['spec']['film'] == 'split' and sp['adopted']['film'] == 'scout', 'the adopted take says where from')
        check(open(film.take(s01, 'refit', 'probe') + '.mkv').read().startswith('scout s01'), 'with its footage')
        sx2 = scout.shot('x02')
        write_take(scout.take(sx2, 'stock', 'probe'), scout, sx2, 'stock', 'probe')
        rc = adopt(os.path.join(films, 'scout'), ['x02'], os.path.join(films, 'split'), as_id='s02')
        check(rc == 0 and cli.take_state(film, s02, 'stock', 'probe', True) == 'ok', '--as adopts under another id')
        write_take(scout.take(sx2, 'refit', 'probe'), scout, sx2, 'refit', 'probe')
        rc = adopt(os.path.join(films, 'scout'), ['x02'], os.path.join(films, 'split'), as_id='s01', force=True)
        check(rc == 1 and open(film.take(s01, 'refit', 'probe') + '.mkv').read().startswith('scout s01'),
              'a take whose spec differs (here its resolution) is not adopted')
        rc = adopt('scout', ['s01'], os.path.join(films, 'split'), force=True)
        check(rc == 0, 'the source may be a film name only (a board on another branch)')

        # -------------------------------------------------- the reference catalogue
        refs = film.references()
        check(sorted(refs) == ['bad01', 'ex01', 'fleet01'], 'the catalogue is every .md of films/reference/ but README')
        r = refs['fleet01']
        check(r.path == os.path.join(data, 'films', 'scout', 'final', 'm01', 'refit.mkv')
              and (r.in_point, r.out_point) == (12.5, 20.0), 'a reference resolves to a take and its in and out')
        s04 = film.shot('s04')
        check(film.footage(s04, 'probe') == r.path and film.footage_span(s04) == (12.5, 20.0),
              'From: ref:<id> is that file, from its In')
        check(not [m for s, m in sb.check(film) if s is s04], 'a shot using a good reference checks clean')
        s04.fields['From'] = 'ref:ex01'
        check(any('example' in m for s, m in sb.check(film) if s is s04), 'an example reference is reported')
        s04.fields['From'] = 'ref:nope'
        check(any('no such entry' in m for s, m in sb.check(film) if s is s04), 'an unknown reference is reported')
        s04.fields['From'] = 'ref:bad01'
        probs = [m for s, m in sb.check(film) if s is None and 'bad01' in m]
        check(any('Take or a File' in m for m in probs) and any('not after In' in m for m in probs),
              'a used reference with no source and Out before In is reported')
        real = sb.load_references(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                               'reference'))
        check(real and all(x.quality == 'example' or not sb.check_reference(x) for x in real.values())
              and any(x.quality == 'example' for x in real.values()),
              'films/reference/ parses, and its example is marked as one')


def main():
    with tempfile.TemporaryDirectory() as repo:
        git(repo, 'init', '-q')
        for path in ('render3d/src/lib.rs', 'testbench/scene/scene.c', 'testbench/scene/install', 'a2test',
                     'gui-installer/installer.py', 'install', 'README.md', 'render3d/README.md',
                     'render3d/perf/sky-shape.py', 'films/codetest/storyboard.md'):
            write(repo, path, f'{path} 1\n')
        write(repo, 'films/codetest/storyboard.md', BOARD)
        commit(repo, 'one')
        film = sb.parse(os.path.join(repo, 'films/codetest/storyboard.md'))
        t01, d01 = film.shot('t01'), film.shot('d01')

        def take(shot, variant):
            code._memo.clear()
            return code.record(film, shot, variant, repo)

        def state(spec, shot, variant):
            code._memo.clear()          # each check is a new command
            return code.check(spec, film, shot, variant, repo)

        check(code.scope(film, t01, 'stock', repo) == ('a2env.py', 'a2env.sh', 'a2mod', 'a2test', 'testbench',
                                                       'testbench/scene'),
              'the stock variant depends on the bench and its installs only')
        check(code.scope(film, t01, 'refit', repo) == ('.',), 'refit (--install .) on the whole repository')
        check(code.scope(film, d01, 'desktop', repo) == ('.',), 'a desktop shot on the whole repository')

        stock, refit, desk = take(t01, 'stock'), take(t01, 'refit'), take(d01, 'desktop')
        check(refit['code'] == '' and refit['dirty'] is False, 'a clean tree records no diff')
        check(state(refit, t01, 'refit') == ('ok', None), 'a take is current as filmed')

        write(repo, 'README.md', 'README.md 2\n')
        write(repo, 'render3d/README.md', 'render3d/README.md 2\n')
        write(repo, 'films/codetest/notes.txt', 'a note\n')
        write(repo, 'render3d/perf/sky-shape.py', 'tuned\n')
        commit(repo, 'docs, films and perf tools')
        check(state(refit, t01, 'refit') == ('ok', None), 'docs, films/ and render3d/perf/ keep it current')

        write(repo, 'render3d/src/lib.rs', 'the nebula rework\n')
        check(state(refit, t01, 'refit')[0] == 'code'
              and 'uncommitted' in state(refit, t01, 'refit')[1], 'an uncommitted change to the code makes it stale')
        c2 = commit(repo, 'nebulae')
        st, why = state(refit, t01, 'refit')
        check(st == 'code' and why == f'code changed since {refit["commit"][:7]} (render3d)',
              f'a committed change makes it stale, and says where ({why})')
        check(state(stock, t01, 'stock') == ('ok', None), 'the stock variant does not see render3d')
        check(state(desk, d01, 'desktop')[0] == 'code', 'a desktop shot does')

        write(repo, 'testbench/scene/scene.c', 'scene 2\n')
        check(state(stock, t01, 'stock')[0] == 'code', 'the stock variant sees its own install')
        dirty = take(t01, 'stock')
        check(dirty['dirty'] and dirty['code'] and dirty['commit'] == c2, 'a dirty tree records its diff')
        check(state(dirty, t01, 'stock') == ('ok', None), 'a take filmed dirty is current while the tree is so')
        commit(repo, 'scene')
        check(state(dirty, t01, 'stock') == ('ok', None), '... and once the same change is committed')
        write(repo, 'testbench/scene/scene.c', 'scene 3\n')
        check(state(dirty, t01, 'stock')[0] == 'code', '... but not after another change')
        write(repo, 'testbench/scene/scene.c', 'scene 2\n')
        git(repo, 'reset', '-q', '--hard', c2)
        st, why = state(dirty, t01, 'stock')
        check(st == 'code' and 'undone' in why, f'a take filmed dirty is stale once the change is undone ({why})')

        # before films 1.1.0: `git describe --always --dirty` and no diff
        legacy = dict(commit=c2[:7])
        check(state(legacy, t01, 'refit') == ('ok', None), 'an old clean take reads its short commit')
        st, why = state(dict(commit=c2[:7] + '-dirty'), t01, 'refit')
        check(st == 'unknown', f'an old dirty take is unknown ({why})')
        check(state({}, t01, 'refit')[0] == 'unknown', 'a take with no commit is unknown')
        check(state(dict(commit='0123456'), t01, 'refit')[0] == 'unknown', 'a commit not in the repository is unknown')

    boards()
    print(f'\n{len(failed)} failed' if failed else '\nall ok')
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
