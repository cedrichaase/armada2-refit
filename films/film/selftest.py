#!/usr/bin/env python3
"""Pins when a take counts as current (films/README.md, "When a take is current"), on a
throw-away git repository laid out like this one: a take is current until a file it
depends on changes, committed or not; docs, the boards and the stock variant's unrelated
layers do not count; a take filmed from uncommitted changes is current while the tree
still has them; takes from before films 1.1.0 are read as they were written. Starts no
game. Also pins `a2film review` (films/film/review.py) on short clips made by ffmpeg: black,
moving, frozen, a flash, a white square, a take that ends early, silent and clipping sound,
and two variants that are the same. Exit 0 when all of it holds.

    python3 films/film/selftest.py
"""
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from film import code, storyboard as sb  # noqa: E402

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


def main():
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

    print(f'\n{len(failed)} failed' if failed else '\nall ok')
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
