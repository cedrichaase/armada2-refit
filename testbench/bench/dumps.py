"""Frame dumps from inside the game: the render targets Render3D.asi and UI.asi read
back byte for byte on request (render3d/README.md and ui/README.md, "Frame dumps").

    take(session, name)   write both trigger files, wait for the plugins to take them,
                          move the files into the session's dumps/<name>/ and write a
                          PNG preview of each
    read(path)            a PFM or PPM as a numpy array (rows top-down)
    stats(path)           size and min/mean/max of one dump

The dumps (PFM: float RGB; PPM: 8-bit RGB) are the exact values, unlike a grim shot
(after the swapchain and the compositor); they are what `a2test baseline compare` is
meant to read. The PNG previews are for looking at only: a PFM is clamped to [0, 1]
(the scene, whose 1.0 is stock's white) or stretched to its own 99.9th percentile (a
bloom level, too faint to see otherwise)."""
import shutil
import time
from pathlib import Path

import cv2
import numpy as np

# trigger file, the plugin that takes it, the sidecar's plugin name
PLUGINS = (('Render3D.dump', 'Render3D.asi', 'render3d'), ('UI.dump', 'UI.asi', 'ui'))


def read(path):
    """A PFM (float32, H x W x 3) or binary PPM (uint8, H x W x 3), rows top-down, RGB."""
    data = Path(path).read_bytes()
    fields, pos = [], 0
    while len(fields) < 4:                     # magic, width, height, max or scale
        while data[pos:pos + 1].isspace():
            pos += 1
        end = pos
        while not data[end:end + 1].isspace():
            end += 1
        fields.append(data[pos:end].decode())
        pos = end
    pos += 1                                   # the one whitespace after the header
    magic = fields[0]
    w, h = int(fields[1]), int(fields[2])
    if magic == 'P6':
        return np.frombuffer(data, np.uint8, w * h * 3, pos).reshape(h, w, 3)
    if magic in ('PF', 'Pf'):
        ch = 3 if magic == 'PF' else 1
        scale = float(fields[3])
        a = np.frombuffer(data, '<f4' if scale < 0 else '>f4', w * h * ch, pos).reshape(h, w, ch)
        a = a[::-1].astype(np.float32)          # PFM rows are bottom-up
        return np.repeat(a, 3, axis=2) if ch == 1 else a
    raise ValueError(f'{path}: not a PFM or binary PPM ({magic!r})')


def stats(path):
    a = read(path)
    return dict(w=a.shape[1], h=a.shape[0], min=float(a.min()), mean=float(a.mean()), max=float(a.max()),
                finite=bool(np.isfinite(a).all()) if a.dtype != np.uint8 else True)


def preview(path, out):
    """A PNG to look at, beside the dump."""
    a = read(path)
    if a.dtype != np.uint8:
        a = np.nan_to_num(a, nan=0.0, posinf=1.0, neginf=0.0)
        if 'bloom' in Path(path).name:
            top = float(np.percentile(a, 99.9)) or float(a.max()) or 1.0
            a = a / top
        a = (np.clip(a, 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)
    cv2.imwrite(str(out), cv2.cvtColor(np.ascontiguousarray(a), cv2.COLOR_RGB2BGR))
    return out


def _safe(name):
    import re
    return re.sub(r'[^A-Za-z0-9_.-]+', '-', name).strip('-')[:64] or 'dump'


def take(sess, name='dump', timeout=20.0):
    """Ask every installed plugin for a dump called `name`, wait for it, move the files to
    `<session>/dumps/<name>/` with a PNG preview each. Returns (status, text, previews):
    status 'ok' when every installed plugin dumped, 'warn' when some did, 'fail' when
    none did (the text says why)."""
    name = _safe(name)
    g = sess.game_dir
    out_dir = g / 'Dumps'
    pending, notes = [], []
    for trigger, asi, plugin in PLUGINS:
        if not (g / asi).exists():
            notes.append(f'{asi} is not installed: no {plugin} dump')
            continue
        pending.append((trigger, asi, plugin))
    if not pending:
        return 'fail', 'neither Render3D.asi nor UI.asi is installed in this game: nothing can dump', []
    before = set(out_dir.iterdir()) if out_dir.is_dir() else set()
    for trigger, _, _ in pending:
        t = g / trigger
        tmp = t.with_suffix('.tmp')
        tmp.write_text(name)
        tmp.rename(t)                          # the plugin never sees half a name
    end = time.time() + timeout
    done = {}
    while pending and time.time() < end:
        for trigger, asi, plugin in list(pending):
            if (g / trigger).exists():
                continue
            side = sorted(p for p in (out_dir.glob(f'{name}-*-{plugin}.txt') if out_dir.is_dir() else [])
                          if p not in before)
            if side:
                done[plugin] = side[-1]
                pending.remove((trigger, asi, plugin))
        time.sleep(0.2)
    for trigger, asi, plugin in pending:
        t = g / trigger
        taken = not t.exists()
        t.unlink(missing_ok=True)
        log = 'Render3D.log' if plugin == 'render3d' else 'UI.log'
        if taken:
            notes.append(f'{asi} took {trigger} but wrote no sidecar within {timeout:g}s (see {log})')
        elif plugin == 'render3d':
            notes.append(f'{asi} did not take {trigger} within {timeout:g}s: it dumps in a mission\'s main 3D view '
                         f'only (is a mission running, and Render3D enabled? see {log})')
        else:
            notes.append(f'{asi} did not take {trigger} within {timeout:g}s: is the game presenting frames, '
                         f'and UI.asi loaded? (see {log})')
    dest = sess.dir / 'dumps' / name
    dest.mkdir(parents=True, exist_ok=True)
    lines, previews = [], []
    new = sorted(p for p in out_dir.iterdir() if p not in before) if out_dir.is_dir() else []
    for p in new:
        q = dest / p.name
        shutil.move(str(p), q)
        if q.suffix in ('.pfm', '.ppm'):
            st = stats(q)
            lines.append(f'{q.name}: {st["w"]}x{st["h"]} min {st["min"]:.4g} mean {st["mean"]:.4g} '
                         f'max {st["max"]:.4g}' + ('' if st['finite'] else ' (NaN or Inf)'))
            previews.append(preview(q, q.with_suffix(q.suffix + '.png')))
        elif q.suffix == '.txt':
            lines.append(f'{q.name}:\n' + ''.join('    ' + l + '\n' for l in q.read_text(errors='replace').splitlines()))
    status = 'fail' if not done else 'warn' if pending else 'ok'
    text = (f'dump "{name}": {", ".join(sorted(done)) or "nothing"} into {dest}\n' + '\n'.join(notes + lines)).rstrip()
    return status, text, previews
