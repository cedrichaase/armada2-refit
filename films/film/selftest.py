#!/usr/bin/env python3
"""Pins when a take counts as current (films/README.md, "When a take is current"), on a
throw-away git repository laid out like this one: a take is current until a file it
depends on changes, committed or not; docs, the boards and the stock variant's unrelated
layers do not count; a take filmed from uncommitted changes is current while the tree
still has them; takes from before films 1.1.0 are read as they were written. Starts no
game. Exit 0 when all of it holds.

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

    print(f'\n{len(failed)} failed' if failed else '\nall ok')
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
