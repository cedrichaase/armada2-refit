#!/usr/bin/env python3
"""Pins when a take counts as current (films/README.md, "When a take is current"), on a
throw-away git repository laid out like this one: a take is current until a file it
depends on changes, committed or not; docs, the boards and the stock variant's unrelated
layers do not count; a take filmed from uncommitted changes is current while the tree
still has them; takes from before films 1.1.0 are read as they were written. And the
sound fields (films 1.3.0): `Mute:` and `Stems:` inherited from film to section to shot,
in a take's spec only when set, checked, passed to the bench, and an `Audio:` mix of
stems rendered by the animatic from synthetic takes. Starts no game. Exit 0 when all of
it holds.

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
