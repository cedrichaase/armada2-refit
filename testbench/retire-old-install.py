#!/usr/bin/env python3
"""Setup of the `retire-old-install` scenario (testbench/scenarios/retire-old-install.md):

    Setup: testbench/retire-old-install.py          (the bench passes the state file first)

The clone is already the refit install (the scenario runs with `--install .` and
`Assets: none`). This script takes a snapshot of it (A), puts an OLD install over it
(testbench/bench/legacy.py `seed`: HUD, GridLayout and Menus with their inis, the hud
scripts' edits and their backups, the d3d8to9 and DXVK chain, a marked dxvk.conf, the
twelve per-part plugins, MenuScale, the widened loading screen's backups), runs the
checkout's `./install` over that (B) and compares: B must be A, file for file by sha256,
except the logs and the player's overlay. The chain class (d3d8_real.dll, d3d9.dll and
dxvk.conf) is taken out too since ./install passes retire.sh --chain (platform 4.0.0). It writes `RetireCheck.log` into the clone and exits 1 on a
difference; the scenario's steps then check the log and launch the game.
"""
import concurrent.futures
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from bench import config, legacy  # noqa: E402

CHAIN = ('d3d8_real.dll', 'd3d9.dll', 'dxvk.conf')
IGNORE_SUFFIX = ('.log', '.tmp', '.migrated')
IGNORE_NAMES = ('RefitPlayer.ini', 'UIPlayer.ini', 'ARMADA.PRF', 'exception.txt')


def sha(path):
    """sha256 of a file; of a PE file we build (`.asi`, `.dll`) with the header's
    TimeDateStamp zeroed, since two builds of one source differ in the linker's clock."""
    with open(path, 'rb') as f:
        head = f.read(1 << 20)
        if path.suffix in ('.asi', '.dll') and head[:2] == b'MZ' and len(head) > 0x40:
            pe = int.from_bytes(head[0x3c:0x40], 'little')
            if head[pe:pe + 4] == b'PE\0\0' and pe + 12 <= len(head):
                head = head[:pe + 8] + b'\0\0\0\0' + head[pe + 12:]
        h = hashlib.sha256(head)
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def snapshot(game):
    files = [p for p in game.rglob('*') if p.is_file() and not p.is_symlink()]
    files = [p for p in files if p.name not in IGNORE_NAMES and not p.name.endswith(IGNORE_SUFFIX)
             and '.a2mod' not in p.relative_to(game).parts]
    with concurrent.futures.ThreadPoolExecutor(8) as pool:
        digests = list(pool.map(sha, files))
    return {str(p.relative_to(game)): d for p, d in zip(files, digests)}


def main():
    st = json.load(open(sys.argv[1]))
    work = Path(st['work'])
    game = work / 'game'
    out = []

    def say(s):
        print(s, flush=True)
        out.append(s)

    say('retire-old-install: snapshot A (the refit install of the clone)')
    a = snapshot(game)
    say(f'  {len(a)} files')
    say('retire-old-install: an old install over it')
    added = legacy.seed(game, say=say)
    say(f'  seeded: {len(added)} files ({", ".join(added[:6])}, ...)')
    seeded = snapshot(game)
    must_go = [n for n in ('HUD.asi', 'GridLayout.asi', 'Menus.asi', 'MSAA.asi', 'Planets.asi', 'Fog.asi',
                           'MenuScale/MainBk.bmp') if n in seeded]
    backups = [n for n in seeded if n.endswith(('.a2neb-backup', '.a2font-backup')) and n not in a]
    say(f'  the old install is there: {len(must_go)} plugin names, {len(backups)} backups')
    if len(must_go) < 5 or not backups:
        say('FAIL the seed did not put an old install in')
        return finish(game, out, 1)

    # The clone came from the stock cache, so a2mod holds it in stock, and retire.sh leaves
    # the file-based fixes' backups alone then (retire/hud-files.py). A player's old install
    # is not held: take the state away, as an install that never saw a2mod has none.
    (game / '.a2mod' / 'state.json').unlink(missing_ok=True)
    data = work / 'no-assets' if st.get('assets') == 'none' else config.DATA
    data.mkdir(exist_ok=True)
    env = dict(os.environ, A2_GAME=str(game), A2_GAME_DIR=str(game), A2_DATA=str(data),
               XDG_DATA_HOME=str(work / 'xdg'), TMPDIR=str(work))
    installs = st.get('installs') or []
    repo = Path(installs[0]['path']) if installs else config.REPO
    only = ['--only', ','.join(st['layers'])] if st.get('layers') else []
    say(f'retire-old-install: {repo}/install {" ".join(only)}')
    r = subprocess.run([str(repo / 'install'), *only], env=env, cwd=str(repo), text=True,
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (work / 'logs').mkdir(exist_ok=True)
    (work / 'logs' / 'retire-old-install.install.log').write_text(r.stdout)
    say(f'  exit {r.returncode}')
    if r.returncode:
        say('FAIL ./install failed over the old install:\n' + '\n'.join(r.stdout.splitlines()[-15:]))
        return finish(game, out, 1)

    say('retire-old-install: snapshot B and the comparison')
    b = snapshot(game)
    bad = []
    for n in sorted(set(a) | set(b)):
        if n not in b and n.endswith(('.a2neb-backup', '.a2font-backup')) \
                and n.rsplit('.', 1)[0] in a and a.get(n.rsplit('.', 1)[0]) == b.get(n.rsplit('.', 1)[0]):
            continue        # a2mod's own backup of a stock file, dropped beside a file now stock again
        if n not in b:
            bad.append(f'missing after the install: {n}')
        elif n not in a:
            bad.append(f'left behind by the old install: {n}')
        elif a[n] != b[n]:
            bad.append(f'differs from the refit install: {n}')
    for n in CHAIN:
        say(f'  chain class {n}: ' + ('STILL THERE' if n in b else 'removed'))
    if (game / 'MenuScale').exists():
        bad.append('MenuScale/ folder still there')
    for line in bad[:40]:
        say('FAIL ' + line)
    if bad:
        return finish(game, out, 1)
    say(f'retire-old-install: ok ({len(a)} files identical to the refit install)')
    return finish(game, out, 0)


def finish(game, out, code):
    (game / 'RetireCheck.log').write_text('\n'.join(out) + '\n')
    return code


if __name__ == '__main__':
    sys.exit(main())
