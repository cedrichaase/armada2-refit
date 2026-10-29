"""Record one frame of the plugins' gpu calls, for gpu/replay.sh to draw again with no
game (gpu/README.md, "Record and replay").

    take(session, name)   write the trigger Gpu.record, wait for every installed plugin
                          to write its stream, and move the recording, with the frame as
                          the game presented it, into $A2_DATA/replays/<name>/

The plugins record only on Display8's Vulkan backend (Mode=stub, Backend=vulkan;
display/bench-mode.sh). What the recording holds is game-derived (texture contents,
the frame), so it goes to A2_DATA, never into a checkout (hard rule 8)."""
import json
import shutil
import time
from pathlib import Path

from . import config

# the plugins that record, by the stream each writes (its module's file name)
PLUGINS = ('Render3D.asi', 'UI.asi')


def _safe(name):
    import re
    return re.sub(r'[^A-Za-z0-9_.-]+', '-', name).strip('-')[:64] or 'record'


def take(sess, name='record', timeout=180.0):
    """Ask for a recording called `name`; returns (status, text): 'ok' when every installed
    plugin wrote its stream, 'warn' when some did, 'fail' when none did."""
    name = _safe(name)
    g = sess.game_dir
    installed = [p for p in PLUGINS if (g / p).exists()]
    if not installed:
        return 'fail', 'neither Render3D.asi nor UI.asi is installed in this game: nothing records'
    ini = g / 'Display8.ini'
    keys = {}
    if ini.exists():
        for line in ini.read_text(errors='replace').splitlines():
            k, _, v = line.partition('=')
            keys[k.strip().lower()] = v.strip().lower()
    notes = []
    if keys.get('mode') != 'stub' or 'vulkan' not in keys.get('backend', ''):
        notes.append(f'Display8.ini says Mode={keys.get("mode")} Backend={keys.get("backend")}: the plugins record '
                     f'only on Display8\'s Vulkan backend (display/bench-mode.sh stub Backend=vulkan)')
    src = g / 'Dumps' / name
    if src.exists():
        shutil.rmtree(src)
    for p in (g / 'Dumps').glob(f'{name}-game-*') if (g / 'Dumps').is_dir() else []:
        p.unlink()
    trigger = g / 'Gpu.record'
    tmp = trigger.with_suffix('.tmp')
    tmp.write_text(f'{name}\n{time.time_ns()}\n')
    tmp.rename(trigger)                         # the plugins never see half a request
    end = time.time() + timeout
    want = {f'{p}.a2s' for p in installed}
    got = set()
    while time.time() < end:
        # A stream is whole once its sidecar (<module>.txt, written after it) is there.
        got = {p.stem + '.a2s' for p in src.glob('*.asi.txt')} if src.is_dir() else set()
        game = list((g / 'Dumps').glob(f'{name}-game-*-ui.txt')) if 'UI.asi' in installed else [True]
        if want <= got and game:
            break
        time.sleep(0.5)
    trigger.unlink(missing_ok=True)
    if not got:
        logs = ', '.join(p.replace('.asi', '.log') for p in installed)
        return 'fail', '\n'.join([f'no plugin recorded "{name}" within {timeout:g}s (see {logs}: "gpu: record")'] + notes)
    dest = config.DATA / 'replays' / name
    if dest.exists():
        dest = dest.with_name(f'{name}-{time.strftime("%Y%m%d-%H%M%S")}')
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(src), dest)
    for p in sorted((g / 'Dumps').glob(f'{name}-game-*')):
        q = dest / ('game.ppm' if p.suffix == '.ppm' else 'game-' + p.name.split('-')[-1])
        shutil.move(str(p), q)
    # The settings the frame was drawn with, for the replay to be drawn with the same.
    for f in ('Display8.ini', 'dxvk.conf', 'Render3D.ini', 'UI.ini'):
        if (g / f).exists():
            shutil.copy2(g / f, dest / f)
    meta = dict(name=name, session=sess.dir.name, recorded=time.strftime('%Y-%m-%d %H:%M:%S'), res=list(sess.res),
                installs=sess.s.get('installs') or [], streams=sorted(got), clock=sess.s.get('clock'))
    (dest / 'meta.json').write_text(json.dumps(meta, indent=2) + '\n')
    lines = [f'recorded "{name}": {", ".join(sorted(got))} into {dest}']
    missing = sorted(want - got)
    if missing:
        lines.append(f'no stream from {", ".join(m[:-4] for m in missing)}')
    if not (dest / 'game.ppm').exists() and 'UI.asi' in installed:
        lines.append('no game.ppm: UI.asi did not dump the frame it recorded')
    for p in sorted(dest.glob('*.txt')):
        lines.append(f'{p.name}:\n' + ''.join('    ' + l + '\n' for l in p.read_text(errors='replace').splitlines()))
    lines.append(f'replay it: gpu/replay.sh {dest}')
    status = 'warn' if missing or notes else 'ok'
    return status, '\n'.join(notes + lines).rstrip()
