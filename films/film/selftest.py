#!/usr/bin/env python3
"""Pins when a take counts as current (films/README.md, "When a take is current"), on a
throw-away git repository laid out like this one: a take is current until a file it
depends on changes, committed or not; docs, the boards and the stock variant's unrelated
layers do not count; a take filmed from uncommitted changes is current while the tree
still has them; takes from before films 1.1.0 are read as they were written. Starts no
game. Also pins `a2film review` (films/film/review.py) on short clips made by ffmpeg: black,
moving, frozen, a flash, a white square, a take that ends early, silent and clipping sound,
and two variants that are the same. Then the board's own rules, on throw-away boards and
footage: a board split over files with `Include:`, a shot's `Tier:`, the take a cut picks,
`adopt`, and the reference catalogue. And the sound fields (films 1.9.0): `Mute:` and
`Stems:` inherited from film to section to shot, in a take's spec only when set, checked,
passed to the bench, and an `Audio:` mix of stems rendered by the animatic from synthetic
takes. Exit 0 when all of it holds.

    python3 films/film/selftest.py
"""
import argparse
import json
import os
import shutil
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

CUTS = """# Cuts test
Film: cutstest
Probe: 320x180

## One
### c1 · Card
Length: 1.5

```card
[accent] ONE
```

### s1 · A scene
Scene: planet
Variants: stock, refit
Layout: wipe
Length: 2
Transition: dissolve 0.5

## Two
### c2 · Card two
Length: 1.5
Transition: dissolve 0.5

```card
TWO
```

### s2 · Another
Scene: planet
Variants: refit
Length: 2
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


def cuts():
    """Numbered cuts, partial cuts and the shot map, on a board with no takes (slates and
    cards), in a throw-away $A2_DATA. Needs ffmpeg."""
    if not shutil.which('ffmpeg'):
        print('skip  cuts (no ffmpeg)')
        return
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    with tempfile.TemporaryDirectory() as tmp:
        write(tmp, 'cb/storyboard.md', CUTS)
        env = dict(os.environ, A2_DATA=os.path.join(tmp, 'data'))
        d = os.path.join(tmp, 'data', 'films', 'cutstest')

        def cut(*args):
            r = subprocess.run([os.path.join(root, 'a2film'), 'animatic', os.path.join(tmp, 'cb'), *args],
                               env=env, capture_output=True, text=True)
            if r.returncode:
                print(r.stderr[-600:])
            return r.returncode

        def doc(name):
            return json.load(open(os.path.join(d, name)))

        check(cut() == 0 and cut() == 0, 'two cuts are made')
        check(all(os.path.exists(os.path.join(d, f)) for f in
                  ('animatic-probe-rc1.mp4', 'animatic-probe-rc2.mp4', 'animatic-probe.mp4',
                   'animatic-probe-rc2.shots.json', 'animatic-probe.shots.json')),
              'every cut is kept as rc<N>, the latest is animatic-probe.mp4, each with a .shots.json')
        check(os.path.getsize(os.path.join(d, 'animatic-probe.mp4')) ==
              os.path.getsize(os.path.join(d, 'animatic-probe-rc2.mp4')), 'the latest is a copy of the newest rc')
        check(os.stat(os.path.join(d, 'animatic-probe.mp4')).st_ino !=
              os.stat(os.path.join(d, 'animatic-probe-rc2.mp4')).st_ino,
              'the latest shares no file with the rc (an older a2film writes it in place)')
        m = doc('animatic-probe-rc2.shots.json')
        sh = m['shots']
        check([x['id'] for x in sh] == ['c1', 's1', 'c2', 's2'], 'the map lists the shots in order')
        check(sh[0]['owns_start'] == 0 and sh[-1]['owns_end'] == m['timeline_length']
              and all(a['owns_end'] == b['owns_start'] for a, b in zip(sh, sh[1:])),
              'the owned spans tile the cut without gap or overlap')
        check(sh[1]['start'] == 1.0 and sh[1]['owns_start'] == 1.25 and sh[1]['transition']['overlap'] == 0.5,
              'a dissolve overlaps in start/end and is split at its middle in owns_*')
        check(sh[1]['used_variants'] == ['stock', 'refit'] and sh[3]['used_variants'] == ['refit']
              and sh[1]['layout']['type'] == 'wipe' and 'status' in sh[1] and 'takes' in sh[1],
              'the map carries variants, layout, takes and status')
        check(abs(m['duration'] - m['timeline_length']) < 0.3, 'the cut\'s probed duration matches the timeline')
        check(cut('--name', 'tweak') == 0 and os.path.exists(os.path.join(d, 'animatic-probe-tweak.mp4'))
              and cut('--name', 'tweak') != 0, '--name writes that label, and will not overwrite it')

        check(cut('--section', 'two') == 0 and doc('animatic-probe-part-two.shots.json')['kind'] == 'part'
              and [x['id'] for x in doc('animatic-probe-part-two.shots.json')['shots']] == ['c2', 's2'],
              '--section cuts that section to animatic-probe-part-<label>.mp4')
        check(cut('--only', 's2,c1') == 0 and os.path.exists(os.path.join(d, 'animatic-probe-part-s2-c1.mp4')),
              '--only cuts the shots, in the order given')
        check(cut('--from', 's1', '--to', 'c2', '--name', 'mid') == 0
              and [x['id'] for x in doc('animatic-probe-part-mid.shots.json')['shots']] == ['s1', 'c2'],
              '--from/--to cut a range')
        check(cut('c1', 's1') == 0 and os.path.exists(os.path.join(d, 'animatic-probe-c1-s1.mp4')),
              'positional ids keep their old file name')
        check(cut('--section', 'nowhere') != 0 and cut('--only', 'zz') != 0, 'an unknown section or shot is an error')
        check(not os.path.exists(os.path.join(d, 'animatic-probe-rc3.mp4')) and
              doc('animatic-probe.shots.json')['latest_of'] == 'animatic-probe-tweak.mp4',
              'partial cuts do not take an rc number or replace the latest (a named full cut does)')
        log = doc('cuts.json')['cuts']
        check([c['kind'] for c in log][:3] == ['rc', 'rc', 'named'] and log[1]['rc'] == 2
              and 'part' in {c['kind'] for c in log} and log[1]['shots'][1]['takes'] == {'stock': None, 'refit': None},
              'cuts.json logs every cut: rc, date, duration, board hash, shots and their takes')


class FakeShot:
    """What review.analyse reads of a shot."""
    kind, layout = 'scene', ('wipe', [])
    players = None

    def __init__(self, length, in_=0.0, audio='game'):
        self.id, self.length, self._in, self._audio = 'x01', length, in_, audio

    def in_point(self, variant):
        return self._in

    def get(self, key, default=None):
        return self._audio if key == 'Audio' else default


def clip(path, audio):
    """7.1 s: black 1 s, motion 2 s, a flat blue 2 s, a white flash 0.1 s, blue 1 s, grey with a white square 1 s."""
    sz = 's=320x180:r=30'
    vf = (f'color=c=black:{sz}:d=1[a];testsrc2={sz}:d=2[b];color=c=0x336699:{sz}:d=2[c];color=c=white:{sz}:d=0.1[d];'
          f'color=c=0x336699:{sz}:d=1[e];color=c=0x404040:{sz}:d=1,'
          f"drawbox=x=100:y=60:w=30:h=30:c=white:t=fill:enable='between(t,0.4,0.7)'[f];"
          '[a][b][c][d][e][f]concat=n=6:v=1:a=0[v]')
    src = 'anullsrc=r=48000:d=7.1' if audio == 'silent' else 'sine=f=440:r=48000:d=7.1,volume=30'
    cmd = ['ffmpeg', '-v', 'error', '-y', '-filter_complex', vf, '-f', 'lavfi', '-i', src,
           '-map', '[v]', '-map', '0:a', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-c:a', 'pcm_s16le', path]
    subprocess.run(cmd, check=True)


def review_checks():
    try:
        from film import review
    except ImportError as e:
        print(f'skip  review ({e})')
        return
    with tempfile.TemporaryDirectory() as tmp:
        silent, loud = os.path.join(tmp, 'silent.mkv'), os.path.join(tmp, 'loud.mkv')
        clip(silent, 'silent')
        clip(loud, 'loud')
        r, fr, gray = review.analyse(silent, FakeShot(10.0), 'refit')
        kinds = {f['kind']: f for f in r['flags']}
        check('black' in kinds and kinds['black']['t0'] < 0.2 and 0.8 < kinds['black']['t1'] < 1.2, 'review: leading black found')
        check('frozen' in kinds and 2.8 < kinds['frozen']['t0'] < 3.4, 'review: the flat stretch is frozen')
        check('flash' in kinds and 4.8 < kinds['flash']['t0'] < 5.4, 'review: the white flash is found')
        check('white' in kinds and 6.0 < kinds['white']['t0'] < 6.9, 'review: the white square is found')
        check('early' in kinds and 2.5 < kinds['early']['t1'] - kinds['early']['t0'] < 3.3, 'review: the take ends early')
        check('silent' in kinds, 'review: silent sound with Audio: game')
        check(r['suggest'].get('In') == 1.2 and 5.5 < r['suggest'].get('Length', 0) < 6.0,
              f'review: suggests In and Length ({r["suggest"]})')
        r2, _, _ = review.analyse(silent, FakeShot(6.0, 1.0, 'none'), 'refit')
        check(not any(f['kind'] in ('silent', 'early') for f in r2['flags']), 'review: no sound check with Audio: none')
        r3, _, g3 = review.analyse(loud, FakeShot(6.0, 1.0), 'refit')
        check(any(f['kind'] == 'clip' for f in r3['flags']) and not any(f['kind'] == 'silent' for f in r3['flags']),
              'review: clipping sound found')
        check(any(1.0 < p < 3.1 for p in r['peaks']) and any(4.8 < p < 5.4 for p in r['peaks']),
              'review: motion peaks are the moving stretch and the flash')
        c = review.compare((r3, g3), (r3, g3), FakeShot(6.0, 1.0))
        check(c and c['mean'] == 0 and any('barely differ' in f for f in c['flags']), 'review: identical variants flagged')
        out = os.path.join(tmp, 'sheet.png')
        r['window'] = [0.0, 7.0]
        review.sheet_png(r, fr, FakeShot(10.0), out)
        check(os.path.getsize(out) > 5000, 'review: a contact sheet is written')


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


def retry_checks():
    """A take that dies with the game is tried once more, and only once."""
    from film import shoot
    tmp = tempfile.mkdtemp()

    def make(outcomes):
        t = shoot.Take.__new__(shoot.Take)
        t.stem, t.tag, t.sid, t.sess = os.path.join(tmp, 'take'), 'x/stock', None, None
        t.calls = 0

        def attempt():
            o = outcomes[t.calls]
            t.calls += 1
            if isinstance(o, Exception):
                raise o
            return o
        t.attempt = attempt
        return t

    t = make([shoot.GameDied('the game died'), True])
    check(t() is True and t.calls == 2, 'a take whose game died is filmed again and can succeed')
    t = make([shoot.GameDied('the game died'), shoot.GameDied('again')])
    check(t() is False and t.calls == 2, '... but only once')
    t = make([RuntimeError('a mistake in the board'), True])
    check(t() is False and t.calls == 1, 'any other failure is not retried')


def main():
    cuts()
    review_checks()
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
    retry_checks()
    audio_tests()
    print(f'\n{len(failed)} failed' if failed else '\nall ok')
    return 1 if failed else 0


AUDIO_BOARD = """# Sound test
Film: soundtest
Stems: music, voice

## Intros
Mute: interface

### m01 · A mission intro
Kind: mission
Scene: none
Args: -nointro a2_borg01
Length: 2
Audio: effects + voice -6

```action
wait 2.5
```

### m02 · Its own sound
Kind: mission
Scene: none
Mute: music, voice
Stems: none
Length: 2

```action
wait 2.5
```

## Plain

### p01 · No fields
Scene: planet
Length: 2

```action
wait 2.5
```

### bad · Wrong fields
Scene: planet
Mute: music, drums
Stems: voice, effects
Audio: music + choir
Length: 2

```action
wait 2.5
```
"""


def mean_db(path):
    r = subprocess.run(['ffmpeg', '-v', 'info', '-i', path, '-af', 'volumedetect', '-f', 'null', '-'],
                       capture_output=True, text=True)
    for line in r.stderr.splitlines():
        if 'mean_volume' in line:
            return float(line.split(':')[1].split()[0])
    return None


def audio_tests():
    sys.path.insert(0, os.path.join(sb.REPO, 'testbench'))
    from bench import session
    from film import edit
    with tempfile.TemporaryDirectory() as tmp:
        write(tmp, 'films/soundtest/storyboard.md', AUDIO_BOARD)
        film = sb.parse(os.path.join(tmp, 'films/soundtest/storyboard.md'))
        m01, m02, p01, bad = (film.shot(i) for i in ('m01', 'm02', 'p01', 'bad'))
        check(m01.stems == ['music', 'voice'] and m01.mute == ['interface'],
              'Stems from the film and Mute from the section reach a shot')
        check(m02.stems == [] and m02.mute == ['music', 'voice'], "a shot's own Mute and Stems: none win")
        check(m01.audio == ('mix', [('effects', 0.0), ('voice', -6.0)]), 'Audio: effects + voice -6 parses')
        check(p01.audio == ('game', []), 'no Audio is the game')
        check(sb.stem_names(m01) == ['music', 'voice', 'effects'], 'a take with stems has an effects stem')
        check(p01.stems == ['music', 'voice'] and p01.mute == [], "a section without them takes the film's")
        plain = sb.parse(os.path.join(tmp, 'films/soundtest/storyboard.md'))
        plain.fields.pop('Stems')
        plain.sections[0].fields.pop('Mute')
        spec = plain.spec(plain.shot('p01'), 'refit', 'probe')
        check('mute' not in spec and 'stems' not in spec, 'a shot with neither has no mute or stems in its spec')
        check(film.spec_hash(m01, 'refit', 'probe') != plain.spec_hash(plain.shot('m01'), 'refit', 'probe'),
              'Mute and Stems change the spec hash, so the take is re-filmed')
        probs = [m for s, m in sb.check(film) if s is bad]
        check(any('drums' in m for m in probs), f'check: an unknown Mute category ({probs})')
        check(any('Stems effects' in m for m in probs), 'check: effects is not a stem of its own')
        check(any('choir' in m for m in probs), 'check: a mix names a stem the shot does not record')
        check(not [m for s, m in sb.check(film) if s in (m01, m02, p01)], 'check: the good shots pass')
        # the bench's side: ARMADA.PRF's third line, and the category names
        prf = os.path.join(tmp, 'ARMADA.PRF')
        with open(prf, 'wb') as f:
            f.write(b'9704529\r\n1 3 1 2 0 0 1 0.28 1 1 0 \r\n20 20 20 20 16 1 1\r\nGPU\r\n'
                    b'0.5 0.5 5 5 5 4 1280 720 32 1 \x00 0 1 0.625 \r\n1 0 1 0 150\r\n')
        session.set_prf_volumes(prf, {'music': 0, 'voice': 0})
        check(open(prf, 'rb').read().split(b'\r\n')[2] == b'0 0 20 20 16 1 1', 'Mute: music, voice in ARMADA.PRF')
        session.set_prf_volumes(prf, {'voice': 20, 'interface': 0})
        check(session.prf_volumes(prf) == dict(music=0, voice=20, effects=20, interface=0),
              'a stem at full volume, the rest kept, the NUL line untouched')
        check(b'\x00' in open(prf, 'rb').read(), '... the embedded NUL survives')
        try:
            session.parse_categories('music, effects', session.STEMS)
            check(False, 'the bench refuses effects as a stem')
        except session.GameError:
            check(True, 'the bench refuses effects as a stem')
        # the animatic: a take with a 440 Hz music stem, a silent effects stem, voice at 880 Hz
        sb.a2env.DATA = tmp
        stem = film.take(m01, 'refit', 'probe')
        os.makedirs(os.path.dirname(stem), exist_ok=True)

        def ff(*a):
            subprocess.run(['ffmpeg', '-v', 'error', '-y', *a], check=True)
        ff('-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo', '-t', '3', stem + '.effects.wav')
        ff('-f', 'lavfi', '-i', 'sine=f=440:r=48000', '-ac', '2', '-t', '3', stem + '.music.wav')
        ff('-f', 'lavfi', '-i', 'sine=f=880:r=48000', '-ac', '2', '-t', '3', stem + '.voice.wav')
        ff('-f', 'lavfi', '-i', 'testsrc=s=320x180:r=30', '-i', stem + '.music.wav', '-i', stem + '.voice.wav',
           '-filter_complex', '[1:a][2:a]amix=inputs=2:normalize=0[a]', '-map', '0:v', '-map', '[a]',
           '-t', '3', '-c:v', 'libx264', '-preset', 'ultrafast', '-c:a', 'pcm_s16le', stem + '.mkv')
        film.fields['Probe'] = '320x180'
        levels = {}
        for audio in ('effects + voice -6', 'effects', 'game', 'music'):
            m01.fields['Audio'] = audio
            out = os.path.join(tmp, 'seg.mkv')
            edit.render_segment(film, m01, 'probe', out, 0, tmp)
            levels[audio] = mean_db(out)
        print('      levels:', levels)
        check(levels['effects'] is not None and levels['effects'] < -80, 'Audio: effects is the silent stem alone')
        check(levels['effects + voice -6'] is not None and levels['effects + voice -6'] > -60,
              'Audio: effects + voice -6 carries the voice')
        check(levels['game'] > levels['music'] > levels['effects + voice -6'],
              'the whole take is louder than one stem, and -6 dB quieter still')
        man = os.path.join(tmp, 'films', 'soundtest')
        check(edit.stems_of(stem + '.mkv').keys() == {'effects', 'music', 'voice'}, 'the manifest finds the stems')
        del man


if __name__ == '__main__':
    sys.exit(main())
