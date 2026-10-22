"""Pixel baselines: named sets of frames kept in $A2_DATA/baselines/, and a pixel diff
against them (testbench/README.md, "Baselines").

    a2test baseline record NAME --from SRC [--shot S]... [--force]
    a2test baseline compare NAME --from SRC [--shot S]... [options]
    a2test baseline diff A B [options]           two files, dirs, sessions or baselines
    a2test baseline list
    a2test baseline show NAME

A SOURCE is resolved, in this order, as
  - an image file (.png, .ppm, .pfm): one shot, named by its stem;
  - a case directory of a run or a session (it has steps.jsonl): its named screenshots,
    the last of each name, and any .ppm/.pfm dump under it;
  - a run directory holding exactly one case;
  - any other directory: every image in it, named by its stem;
  - a session id (live or ended): that session's artifacts directory;
  - the name of an existing baseline.
A stem's numeric prefix (`004-orbit.png`) is not part of the name.

THE NUMBERS

Every frame is compared as RGB (an alpha channel is dropped). For an 8-bit frame (PNG,
PPM) the unit is 1/255: `rmse 0.8` is 0.8/255. A 16-bit file is scaled into that unit.
A PFM is linear float: its RMSE and max are reported in linear values as they are, and
then again after a fixed tone map (Reinhard, x/(1+x) per channel, times 255), in which
the threshold, the pixel count and PSNR are measured, so the same tolerances read the
same way for both kinds. The heat map of a PFM comes from the linear difference.

  rmse      root mean square of the per-channel differences, over every channel of
            every pixel
  max       the largest per-channel difference
  over      pixels where any channel differs by more than --threshold (default 2/255),
            as a count and a percentage of the frame
  psnr      20 log10(255 / rmse) in dB; "inf" when identical

Two frames of different sizes, or an 8-bit frame against a float one, are not
compared: that is a failure of its own, with the reason.
"""
import argparse
import datetime
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

import cv2
import numpy as np

from . import config

EXTS = ('.png', '.ppm', '.pfm')
HDR_EXTS = ('.pfm',)


class BaselineError(Exception):
    pass


def root():
    return Path(os.environ.get('A2_BASELINES') or Path(config.DATA) / 'baselines')


def _safe(name):
    return re.sub(r'[^A-Za-z0-9_.-]+', '-', name).strip('-') or 'shot'


def _shot_name(stem):
    return re.sub(r'^\d+-', '', stem)


def refuse_checkout(path):
    """Hard rule 8: nothing derived from the game is written into a checkout."""
    p = Path(path).expanduser().resolve()
    for d in (p, *p.parents):
        if (d / 'a2env.sh').exists() and (d / '.git').exists():
            raise BaselineError(f'{path} is inside the checkout {d}: frames are game-derived and '
                                'never go into a working tree (CLAUDE.md, hard rule 8)')
    return p


# ---------------------------------------------------------------------- reading frames

class Frame:
    """An image as float32 RGB, H x W x 3. kind 'ldr' (in units of 1/255) or 'hdr'
    (linear, as stored)."""

    def __init__(self, data, kind, path, note=''):
        self.data, self.kind, self.path, self.note = data, kind, Path(path), note

    @property
    def size(self):
        return self.data.shape[1], self.data.shape[0]


def _header_tokens(buf, n):
    """The first n whitespace-separated tokens of a netpbm header (comments skipped),
    and the offset just past the single whitespace byte after the last one."""
    toks, i = [], 0
    while len(toks) < n:
        while i < len(buf) and buf[i:i + 1].isspace():
            i += 1
        if buf[i:i + 1] == b'#':
            while i < len(buf) and buf[i:i + 1] not in (b'\n', b'\r'):
                i += 1
            continue
        j = i
        while j < len(buf) and not buf[j:j + 1].isspace():
            j += 1
        if j == i:
            raise BaselineError('truncated header')
        toks.append(buf[i:j].decode('ascii'))
        i = j
    return toks, i + 1


def read_ppm(path):
    """Binary PPM (P6) or PGM (P5), 8 or 16 bit, as float RGB in units of 1/255."""
    buf = Path(path).read_bytes()
    (magic, w, h, maxval), off = _header_tokens(buf, 4)
    if magic not in ('P6', 'P5'):
        raise BaselineError(f'{path}: not a binary PPM/PGM (magic {magic!r})')
    w, h, maxval = int(w), int(h), int(maxval)
    ch = 3 if magic == 'P6' else 1
    dt = np.dtype('u1') if maxval < 256 else np.dtype('>u2')
    need = w * h * ch * dt.itemsize
    if len(buf) - off < need:
        raise BaselineError(f'{path}: {len(buf) - off} bytes of pixels, {need} expected for {w}x{h}')
    a = np.frombuffer(buf, dt, w * h * ch, off).reshape(h, w, ch).astype(np.float32) * (255.0 / maxval)
    return np.repeat(a, 3, axis=2) if ch == 1 else a


def read_pfm(path):
    """PFM, float RGB ('PF') or grey ('Pf'). The scale's sign is the byte order (negative:
    little-endian) and rows are stored bottom to top; both are undone here."""
    buf = Path(path).read_bytes()
    (magic, w, h, scale), off = _header_tokens(buf, 4)
    if magic not in ('PF', 'Pf'):
        raise BaselineError(f'{path}: not a PFM (magic {magic!r})')
    w, h, scale = int(w), int(h), float(scale)
    ch = 3 if magic == 'PF' else 1
    dt = np.dtype('<f4' if scale < 0 else '>f4')
    need = w * h * ch * 4
    if len(buf) - off < need:
        raise BaselineError(f'{path}: {len(buf) - off} bytes of pixels, {need} expected for {w}x{h}')
    a = np.frombuffer(buf, dt, w * h * ch, off).reshape(h, w, ch)[::-1].astype(np.float32)
    return np.repeat(a, 3, axis=2) if ch == 1 else a


def write_pfm(path, rgb, little=True):
    """float RGB (H x W x 3, top row first) as PFM: rows bottom to top."""
    a = np.ascontiguousarray(np.asarray(rgb, np.float32)[::-1])
    with open(path, 'wb') as f:
        f.write(f'PF\n{a.shape[1]} {a.shape[0]}\n{-1.0 if little else 1.0}\n'.encode())
        f.write(a.astype('<f4' if little else '>f4').tobytes())


def write_ppm(path, rgb):
    """uint8 RGB (H x W x 3) as P6."""
    a = np.ascontiguousarray(np.asarray(rgb, np.uint8))
    with open(path, 'wb') as f:
        f.write(f'P6\n{a.shape[1]} {a.shape[0]}\n255\n'.encode())
        f.write(a.tobytes())


def load(path):
    path = Path(path)
    ext = path.suffix.lower()
    if ext == '.pfm':
        return Frame(read_pfm(path), 'hdr', path)
    if ext == '.ppm' or ext == '.pgm':
        return Frame(read_ppm(path), 'ldr', path)
    img = cv2.imread(str(path), cv2.IMREAD_UNCHANGED)
    if img is None:
        raise BaselineError(f'cannot read {path}')
    note = ''
    if img.ndim == 2:
        img = img[:, :, None].repeat(3, axis=2)
    elif img.shape[2] == 4:
        img = img[:, :, :3]
        note = 'alpha dropped'
    scale = 255.0 / 65535.0 if img.dtype == np.uint16 else 1.0
    return Frame(img[:, :, ::-1].astype(np.float32) * scale, 'ldr', path, note)


def tonemap(x):
    """The fixed tone map for comparing float frames: Reinhard per channel, to 0..255."""
    x = np.maximum(x, 0)
    return x / (1.0 + x) * 255.0


def display(frame):
    """uint8 BGR to look at: an 8-bit frame as it is, a float one tone-mapped and
    gamma-encoded (1/2.2)."""
    if frame.kind == 'hdr':
        v = (tonemap(frame.data) / 255.0) ** (1 / 2.2) * 255.0
    else:
        v = frame.data
    return np.clip(v + 0.5, 0, 255).astype(np.uint8)[:, :, ::-1]


# ---------------------------------------------------------------------- comparing

def metrics(a, b, threshold=2.0):
    """The numbers for two frames of the same size and kind (see the module's doc)."""
    if a.size != b.size:
        return dict(ok=False, error=f'sizes differ: {a.size[0]}x{a.size[1]} against {b.size[0]}x{b.size[1]}')
    if a.kind != b.kind:
        return dict(ok=False, error=f'an 8-bit frame against a float one ({a.path.name}: {a.kind}, '
                                    f'{b.path.name}: {b.kind})')
    out = dict(ok=True, kind=a.kind, width=a.size[0], height=a.size[1], threshold=threshold)
    if a.kind == 'hdr':
        lin = np.abs(a.data.astype(np.float64) - b.data.astype(np.float64))
        out['linear_rmse'] = float(np.sqrt(np.mean(lin ** 2)))
        out['linear_max'] = float(lin.max())
        d = np.abs(tonemap(a.data.astype(np.float64)) - tonemap(b.data.astype(np.float64)))
    else:
        d = np.abs(a.data.astype(np.float64) - b.data.astype(np.float64))
    rmse = float(np.sqrt(np.mean(d ** 2)))
    per_px = d.max(axis=2)
    over = int(np.count_nonzero(per_px > threshold))
    n = per_px.size
    out.update(rmse=rmse, max=float(d.max()), over=over, over_pct=100.0 * over / n, pixels=n,
               psnr=(math.inf if rmse == 0 else 20 * math.log10(255.0 / rmse)))
    return out


def within(m, rmse=None, max_over=None):
    """Whether the numbers are inside the tolerances. `max_over`: a count (int) or a
    percentage (a string ending in %). Returns (ok, why)."""
    if not m.get('ok'):
        return False, m.get('error', 'not compared')
    why = []
    if rmse is not None and m['rmse'] > rmse:
        why.append(f'rmse {m["rmse"]:.3f} > {rmse:g}')
    if max_over is not None:
        lim, pct = parse_max_over(max_over)
        val = m['over_pct'] if pct else m['over']
        if val > lim:
            why.append(f'{m["over"]} px ({m["over_pct"]:.3f}%) over {m["threshold"]:g}/255 > '
                       f'{lim:g}{"%" if pct else " px"}')
    return (not why), '; '.join(why)


def parse_max_over(v):
    s = str(v).strip()
    if s.endswith('%'):
        return float(s[:-1]), True
    return float(s), False


def fmt(m):
    if not m.get('ok'):
        return m.get('error', '?')
    psnr = 'inf' if math.isinf(m['psnr']) else f'{m["psnr"]:.1f}'
    s = (f'rmse {m["rmse"]:.3f}/255, max {m["max"]:.1f}/255, '
         f'{m["over"]} px over {m["threshold"]:g}/255 ({m["over_pct"]:.3f}%), psnr {psnr} dB')
    if m['kind'] == 'hdr':
        s = (f'linear rmse {m["linear_rmse"]:.5g}, linear max {m["linear_max"]:.5g}; '
             f'tone-mapped: {s}')
    return s


def heatmap(a, b, out):
    """The difference as a picture: the largest per-channel difference at each pixel,
    from the linear values for a float frame, on a square-root scale up to the frame's
    own maximum (so a single changed pixel still shows), over a dimmed copy of `a`.
    Identical pixels stay grey. The legend under it gives the scale."""
    d = np.abs(a.data.astype(np.float64) - b.data.astype(np.float64)).max(axis=2)
    top = float(d.max())
    grey = cv2.cvtColor(display(a), cv2.COLOR_BGR2GRAY)
    base = cv2.cvtColor((grey * 0.3).astype(np.uint8), cv2.COLOR_GRAY2BGR)
    if top > 0:
        v = np.sqrt(d / top)
        col = cv2.applyColorMap(np.clip(v * 255, 0, 255).astype(np.uint8), cv2.COLORMAP_INFERNO)
        # anything that differs at all is at least visible against the grey
        col[d > 0] = np.maximum(col[d > 0], 60)
        base = np.where((d > 0)[:, :, None], col, base)
    H, W = base.shape[:2]
    bar = np.full((28, W, 3), 24, np.uint8)
    unit = 'linear' if a.kind == 'hdr' else '/255'
    ramp_w = min(256, W // 3)
    ramp = (np.sqrt(np.linspace(0, 1, ramp_w))[None, :].repeat(14, 0) * 255).astype(np.uint8)
    ramp = cv2.applyColorMap(ramp, cv2.COLORMAP_INFERNO)
    bar[7:21, 6:6 + ramp_w] = ramp
    text = (f'0 .. {top:.4g} {unit} (sqrt scale); grey = identical' if top > 0 else 'identical')
    cv2.putText(bar, text, (12 + ramp_w, 19), cv2.FONT_HERSHEY_SIMPLEX, 0.45, (230, 230, 230), 1, cv2.LINE_AA)
    cv2.imwrite(str(out), np.vstack([base, bar]))
    return Path(out)


def side_by_side(a, b, heat, out, labels=('baseline', 'this'), max_w=3840):
    """A | B | heat map in one picture, scaled to at most max_w wide."""
    ims = [display(a), display(b), cv2.imread(str(heat))[:a.size[1]]]
    row = np.hstack(ims)
    k = min(1.0, max_w / row.shape[1])
    if k < 1:
        row = cv2.resize(row, None, fx=k, fy=k, interpolation=cv2.INTER_AREA)
    cell = row.shape[1] // 3
    top = np.full((26, row.shape[1], 3), 24, np.uint8)
    for i, t in enumerate((labels[0], labels[1], 'difference')):
        cv2.putText(top, t, (i * cell + 8, 18), cv2.FONT_HERSHEY_SIMPLEX, 0.5, (230, 230, 230), 1, cv2.LINE_AA)
    cv2.imwrite(str(out), np.vstack([top, row]))
    return Path(out)


def flicker(a, b, out):
    """A two-frame GIF flipping between them (ImageMagick); None if it is not there."""
    if not shutil.which('magick'):
        return None
    tmp = [Path(out).with_suffix(f'.{i}.png') for i in (0, 1)]
    cv2.imwrite(str(tmp[0]), display(a))
    cv2.imwrite(str(tmp[1]), display(b))
    r = subprocess.run(['magick', '-delay', '50', str(tmp[0]), str(tmp[1]), '-loop', '0', str(out)],
                       capture_output=True)
    for t in tmp:
        t.unlink(missing_ok=True)
    return Path(out) if r.returncode == 0 else None


def compare_files(pa, pb, out_dir, name, threshold=2.0, gif=False, labels=('baseline', 'this')):
    """Compare two image files; write the heat map (and side by side, GIF) into out_dir.
    Returns the metrics dict with the written files under 'files'."""
    out_dir = refuse_checkout(out_dir)
    try:
        a, b = load(pa), load(pb)
    except BaselineError as e:
        return dict(ok=False, error=str(e), files={})
    m = metrics(a, b, threshold)
    m['a'], m['b'] = str(pa), str(pb)
    m['files'] = {}
    if m['ok']:
        out_dir.mkdir(parents=True, exist_ok=True)
        stem = _safe(name)
        h = heatmap(a, b, out_dir / f'{stem}-heat.png')
        m['files']['heat'] = str(h)
        m['files']['side_by_side'] = str(side_by_side(a, b, h, out_dir / f'{stem}-side.png', labels))
        if gif:
            g = flicker(a, b, out_dir / f'{stem}-flicker.gif')
            if g:
                m['files']['flicker'] = str(g)
    notes = [x for x in (a.note, b.note) if x]
    if notes:
        m['note'] = ', '.join(sorted(set(notes)))
    return m


# ---------------------------------------------------------------------- sources

def _case_meta(case_dir):
    """What the case's step log says about it: session, resolution, installs, scene."""
    meta, scene = {}, None
    p = Path(case_dir) / 'steps.jsonl'
    if not p.exists():
        return meta
    for line in p.read_text().splitlines():
        try:
            rec = json.loads(line)
        except ValueError:
            continue
        if rec.get('kind') == 'meta':
            meta.update(rec.get('meta') or {})
        m = re.search(r'bench-setup\.sh\s+(\S+)', rec.get('text') or '')
        if m and rec.get('kind') == 'action':
            scene = m.group(1)
    # `SCENE=<name> ... --install testbench/scene`: the scene's install says which file
    plog = Path(case_dir) / 'logs' / 'prepare.log'
    if plog.exists():
        for m in re.finditer(r'installed Scene\.asi into .*?, scene \S*?/scenes/([^/\s]+)\.ini',
                             plog.read_text(errors='replace')):
            scene = m.group(1)
    if scene:
        meta['scene'] = scene
    return meta


def _case_shots(case_dir):
    """{name: path}: the case's named screenshots (last of each name wins), then any
    PPM/PFM dump under it by stem."""
    case_dir = Path(case_dir)
    shots = {}
    for line in (case_dir / 'steps.jsonl').read_text().splitlines():
        try:
            rec = json.loads(line)
        except ValueError:
            continue
        if rec.get('kind') == 'shot' and rec.get('shots'):
            p = case_dir / rec['shots'][0]
            if p.exists():
                shots[rec['text']] = p
    for p in sorted(case_dir.rglob('*'), key=lambda p: (p.stat().st_mtime, str(p))):
        if p.suffix.lower() in ('.ppm', '.pfm') and p.is_file():
            shots[_shot_name(p.stem)] = p
    return shots


def _dir_images(d):
    out = {}
    for p in sorted(Path(d).iterdir(), key=lambda p: (p.stat().st_mtime, str(p))):
        if p.is_file() and p.suffix.lower() in EXTS:
            out[_shot_name(p.stem)] = p
    return out


def _session_dir(sid):
    st = config.CACHE / 'sessions' / sid / 'session.json'
    if st.is_file():
        try:
            return Path(json.loads(st.read_text())['dir'])
        except (ValueError, KeyError):
            pass
    if not re.fullmatch(r'\d{8}-\d{6}[\d-]*', sid):
        return None
    for p in config.RESULTS.glob('*/*/*/steps.jsonl'):
        try:
            first = json.loads(p.read_text().split('\n', 1)[0])
        except (ValueError, OSError):
            continue
        if (first.get('meta') or {}).get('session') == sid:
            return p.parent
    return None


def resolve(src):
    """A source (see the module's doc) -> dict(kind, path, shots={name: path}, meta)."""
    p = Path(src).expanduser()
    if p.is_file():
        if p.suffix.lower() not in EXTS:
            raise BaselineError(f'{p}: not a .png, .ppm or .pfm')
        return dict(kind='file', path=p, shots={_shot_name(p.stem): p}, meta={})
    if p.is_dir():
        if (p / 'steps.jsonl').exists():
            return dict(kind='case', path=p, shots=_case_shots(p), meta=_case_meta(p))
        if (p / 'meta.json').exists() and p.parent == root():
            return _baseline_source(p.name)
        # a scenario's directory in a run (cases one level down), or the run's (two)
        cases = sorted({x.parent for x in [*p.glob('*/steps.jsonl'), *p.glob('*/*/steps.jsonl')]})
        if len(cases) == 1:
            return dict(kind='case', path=cases[0], shots=_case_shots(cases[0]), meta=_case_meta(cases[0]))
        if len(cases) > 1:
            raise BaselineError(f'{p} holds {len(cases)} cases; name one:\n' +
                                '\n'.join(f'  {c}' for c in cases))
        return dict(kind='dir', path=p, shots=_dir_images(p), meta={})
    sd = _session_dir(str(src))
    if sd is not None:
        return dict(kind='session', path=sd, shots=_case_shots(sd), meta=_case_meta(sd))
    if (root() / str(src) / 'meta.json').exists():
        return _baseline_source(str(src))
    raise BaselineError(f'{src}: not an image, a directory, a session id or a baseline '
                        f'(baselines: {root()})')


def _baseline_source(name):
    meta = read_meta(name)
    d = root() / name
    return dict(kind='baseline', path=d, meta=meta,
                shots={k: d / v['file'] for k, v in meta.get('shots', {}).items()})


def pick(shots, wanted, where):
    if not wanted:
        return dict(shots)
    missing = [w for w in wanted if w not in shots]
    if missing:
        raise BaselineError(f'no shot {", ".join(repr(w) for w in missing)} in {where} '
                            f'(it has: {", ".join(sorted(shots)) or "none"})')
    return {w: shots[w] for w in wanted}


# ---------------------------------------------------------------------- baselines

def read_meta(name):
    p = root() / name / 'meta.json'
    if not p.exists():
        raise BaselineError(f'no baseline {name!r} in {root()}')
    return json.loads(p.read_text())


def _commit():
    r = subprocess.run(['git', '-C', str(config.REPO), 'describe', '--always', '--dirty', '--abbrev=10'],
                       capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else None


def _sha256(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def record(name, shots, source_meta=None, source=None, force=False):
    """Copy {shot: path} into baseline `name`, adding to what it holds. A shot already in
    it is replaced only with force. Each shot keeps its own provenance."""
    if not re.fullmatch(r'[A-Za-z0-9_.@-]+', name):
        raise BaselineError(f'baseline name {name!r}: letters, digits and _ . @ - only')
    if not shots:
        raise BaselineError('nothing to record: the source has no shots')
    d = refuse_checkout(root() / name)
    meta = read_meta(name) if (d / 'meta.json').exists() else dict(name=name, created=_now(), shots={})
    clash = [s for s in shots if s in meta['shots']]
    if clash and not force:
        raise BaselineError(f'baseline {name!r} already has {", ".join(repr(c) for c in clash)}; '
                            '--force replaces them')
    d.mkdir(parents=True, exist_ok=True)
    sm = source_meta or {}
    for shot, p in shots.items():
        f = load(p)
        fname = _safe(shot) + Path(p).suffix.lower()
        old = meta['shots'].get(shot)
        if old and old['file'] != fname:
            (d / old['file']).unlink(missing_ok=True)
        shutil.copyfile(p, d / fname)
        meta['shots'][shot] = dict(
            file=fname, width=f.size[0], height=f.size[1], kind=f.kind, recorded=_now(),
            sha256=_sha256(d / fname), source=str(p), commit=_commit(),
            installs=sm.get('installs') or [], session=sm.get('session'), scene=sm.get('scene'),
            resolution=sm.get('resolution') or f'{f.size[0]}x{f.size[1]}', mod=sm.get('mod'),
            **({'from': str(source)} if source else {}))
    meta['updated'] = _now()
    # the latest record's provenance, at the top for `show`
    last = meta['shots'][next(reversed(shots))]
    for k in ('commit', 'installs', 'resolution', 'scene', 'session', 'mod'):
        meta[k] = last.get(k)
    (d / 'meta.json').write_text(json.dumps(meta, indent=1))
    return d


def _now():
    return datetime.datetime.now().isoformat(timespec='seconds')


def compare(a_shots, b_shots, out_dir, threshold=2.0, rmse=None, max_over=None, gif=False,
            labels=('baseline', 'this')):
    """Each shot of `a_shots` against the same name in `b_shots`. Returns a list of
    per-shot results, each with 'pass' and 'why'."""
    res = []
    for name in a_shots:
        if name not in b_shots:
            res.append(dict(shot=name, ok=False, error=f'no shot {name!r} on the other side', files={},
                            **{'pass': False, 'why': 'missing'}))
            continue
        m = compare_files(a_shots[name], b_shots[name], out_dir, name, threshold, gif, labels)
        ok, why = within(m, rmse, max_over)
        res.append(dict(m, shot=name, **{'pass': ok, 'why': why}))
    return res


def write_summary(out_dir, results, title):
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / 'compare.json').write_text(json.dumps(
        dict(title=title, results=[{k: (None if isinstance(v, float) and math.isinf(v) else v)
                                    for k, v in r.items()} for r in results]), indent=1))
    rows = []
    for r in results:
        imgs = ''.join(f'<a href="{os.path.relpath(v, out_dir)}"><img src="{os.path.relpath(v, out_dir)}"></a>'
                       for k, v in r.get('files', {}).items() if k != 'flicker')
        rows.append(f'<h2>{_h(r["shot"])} <small>{"PASS" if r["pass"] else "FAIL"}</small></h2>'
                    f'<p>{_h(fmt(r))}{(" -- " + _h(r["why"])) if r["why"] else ""}</p>{imgs}')
    (out_dir / 'index.html').write_text(
        f'<!doctype html><meta charset="utf-8"><title>{_h(title)}</title>'
        '<style>body{font:14px system-ui;background:#111;color:#ddd;margin:16px}'
        'img{max-width:100%;display:block;margin:6px 0}small{color:#999}</style>'
        f'<h1>{_h(title)}</h1>' + '\n'.join(rows))
    return out_dir / 'index.html'


def _h(s):
    import html
    return html.escape(str(s))


# ---------------------------------------------------------------------- scenario steps

def find_in_case(d, name):
    """A case's latest frame called `name`: its screenshot, or a .ppm/.pfm dump."""
    d, safe = Path(d), _safe(name)
    cands = [p for p in d.rglob('*') if p.is_file() and p.suffix.lower() in EXTS
             and p.parent.name != 'baseline' and _shot_name(p.stem) == safe]
    return max(cands, key=lambda p: (p.stat().st_mtime, str(p))) if cands else None


def _how_to_record(shot, base):
    return (f'record it with the step `Record the screenshot "{shot}" as baseline "{base}"`, or '
            f'`./a2test baseline record {base} --from <case dir or session id> --shot "{shot}"`')


def step_record(c, shots, base):
    """`Record the screenshot "NAME" as baseline "B"` (shots=[NAME]) or `Record every
    screenshot as baseline "B"` (shots=None). A scenario replaces what it records."""
    if shots is None:
        found = {_shot_name(p.stem): p for p in sorted(Path(c.sess.dir).glob('shots/*.png'))}
        found = {k: v for k, v in found.items()
                 if not k.startswith(('check-', 'before-click', 'timeout-', 'at-failure'))}
    else:
        found = {s: find_in_case(c.sess.dir, s) for s in shots}
        missing = [s for s, p in found.items() if p is None]
        if missing:
            raise BaselineError(f'no screenshot called {", ".join(repr(m) for m in missing)} yet')
    meta = _case_meta(c.dir)
    meta.setdefault('resolution', config.res_name(c.res))
    meta.setdefault('mod', c.mod)
    d = record(base, found, meta, source=str(c.dir), force=True)
    return f'recorded {", ".join(found) or "nothing"} as baseline {base} ({d})'


def step_expect(c, shot, base, rmse=None, pct=None, threshold=2.0):
    """`Expect the screenshot "NAME" matches baseline "B" [within RMSE X] [with at most N%
    of pixels over T]`. Returns (ok or None, detail, shots, measure)."""
    mine = find_in_case(c.sess.dir, shot)
    if mine is None:
        return False, f'no screenshot called "{shot}" in this case yet', [], None
    try:
        meta = read_meta(base)
    except BaselineError:
        return False, f'there is no baseline "{base}" ({root()}); {_how_to_record(shot, base)}', [mine], None
    entry = meta['shots'].get(shot)
    if entry is None:
        return False, (f'baseline "{base}" has no shot "{shot}" (it has: {", ".join(meta["shots"]) or "none"}); '
                       f'{_how_to_record(shot, base)}'), [mine], None
    m = compare_files(root() / base / entry['file'], mine, Path(c.sess.dir) / 'baseline',
                      f'{shot}-vs-{base}', threshold, labels=(f'baseline {base}', 'this case'))
    max_over = f'{pct}%' if pct is not None else ('0' if rmse is None else None)
    ok, why = within(m, rmse, max_over)
    files = [Path(f) for k, f in m.get('files', {}).items() if k in ('heat', 'side_by_side')]
    measure = {k: (None if isinstance(v, float) and math.isinf(v) else v) for k, v in m.items()
               if k not in ('files',)}
    tol = ', '.join(x for x in ((f'rmse <= {rmse:g}' if rmse is not None else ''),
                                (f'at most {max_over}{"" if str(max_over).endswith("%") else " px"} over '
                                 f'{threshold:g}/255' if max_over is not None else '')) if x)
    detail = f'{fmt(m)}; tolerance: {tol}' + (f' -- {why}' if why and why != m.get('error') else '')
    if entry.get('commit'):
        detail += f'\nbaseline recorded {entry.get("recorded")} at {entry.get("commit")}'
    return ok, detail, files or [mine], measure


# ---------------------------------------------------------------------- command line

def _add_compare_opts(ap):
    ap.add_argument('--shot', action='append', default=[], help='only this shot (repeatable)')
    ap.add_argument('--out', help='where the heat maps go (default ~/.cache/a2test/baselines/...)')
    ap.add_argument('--threshold', type=float, default=2.0, help='per-channel difference, in 1/255, '
                    'that counts a pixel as over (default 2)')
    ap.add_argument('--rmse', type=float, default=None, help='fail above this RMSE (1/255)')
    ap.add_argument('--max-over', default=None, help='fail above this many pixels over the threshold, '
                    'or this percentage with a trailing %% (default 0 when --rmse is not given)')
    ap.add_argument('--gif', action='store_true', help='also a two-frame flicker GIF per shot')


def _default_out(tag):
    stamp = datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
    return config.CACHE / 'baselines' / f'{stamp}-{_safe(tag)}'


def _run_compare(a, b, args, title, labels):
    out = refuse_checkout(args.out or _default_out(title))
    # no tolerance given: no pixel may be over the threshold
    max_over = '0' if args.rmse is None and args.max_over is None else args.max_over
    results = compare(a, b, out, args.threshold, args.rmse, max_over, args.gif, labels)
    w = max([len(r['shot']) for r in results] + [4])
    for r in results:
        print(f'{"PASS" if r["pass"] else "FAIL"}  {r["shot"]:<{w}}  {fmt(r)}' +
              (f'\n      {r["why"]}' if r['why'] and r['why'] != r.get('error') else '') +
              (f'\n      heat map: {r["files"]["heat"]}' if r.get('files', {}).get('heat') else ''))
    page = write_summary(out, results, title)
    print(f'\n{sum(r["pass"] for r in results)}/{len(results)} within tolerance; {page}')
    return 0 if results and all(r['pass'] for r in results) else 1


def main(argv):
    if not argv or argv[0] in ('-h', '--help', 'help'):
        print(__doc__.split('\nTHE NUMBERS')[0].rstrip())
        return 0
    sub, rest = argv[0], argv[1:]
    try:
        if sub == 'record':
            ap = argparse.ArgumentParser(prog='a2test baseline record')
            ap.add_argument('name')
            ap.add_argument('--from', dest='src', required=True, nargs='+')
            ap.add_argument('--shot', action='append', default=[])
            ap.add_argument('--force', action='store_true', help='replace shots the baseline already has')
            a = ap.parse_args(rest)
            shots, meta = {}, {}
            for s in a.src:
                r = resolve(s)
                shots.update(r['shots'])
                meta = meta or r['meta']
            shots = pick(shots, a.shot, ', '.join(a.src))
            d = record(a.name, shots, meta, ' '.join(a.src), a.force)
            print(f'baseline {a.name}: {", ".join(shots)} -> {d}')
            return 0
        if sub == 'compare':
            ap = argparse.ArgumentParser(prog='a2test baseline compare')
            ap.add_argument('name')
            ap.add_argument('--from', dest='src', required=True, nargs='+')
            _add_compare_opts(ap)
            a = ap.parse_args(rest)
            base = _baseline_source(a.name)['shots']
            mine = {}
            for s in a.src:
                mine.update(resolve(s)['shots'])
            names = a.shot or [n for n in base if n in mine] or list(base)
            base = pick(base, names, f'baseline {a.name}')
            return _run_compare(base, mine, a, a.name, (f'baseline {a.name}', 'this'))
        if sub == 'diff':
            ap = argparse.ArgumentParser(prog='a2test baseline diff')
            ap.add_argument('a')
            ap.add_argument('b')
            _add_compare_opts(ap)
            a = ap.parse_args(rest)
            ra, rb = resolve(a.a), resolve(a.b)
            sa, sb = ra['shots'], rb['shots']
            if len(sa) == 1 and len(sb) == 1:
                sb = {next(iter(sa)): next(iter(sb.values()))}       # one frame a side: names need not match
            names = a.shot or [n for n in sa if n in sb]
            if not names:
                raise BaselineError(f'no shot name in common (A: {", ".join(sa) or "none"}; '
                                    f'B: {", ".join(sb) or "none"})')
            return _run_compare(pick(sa, names, a.a), sb, a, 'diff', ('A', 'B'))
        if sub == 'list':
            d = root()
            for m in sorted(d.glob('*/meta.json')) if d.exists() else []:
                meta = json.loads(m.read_text())
                print(f'{meta["name"]:<24} {meta.get("updated", meta.get("created", "")):<20} '
                      f'{meta.get("resolution") or "":<10} {len(meta["shots"])} shot(s): '
                      f'{", ".join(meta["shots"])}')
            print(f'({d})')
            return 0
        if sub == 'show':
            if not rest:
                raise BaselineError('show which baseline? (a2test baseline list)')
            meta = read_meta(rest[0])
            print(f'baseline {meta["name"]}  ({root() / meta["name"]})\n  created {meta.get("created")}, '
                  f'updated {meta.get("updated")}')
            for k, v in meta['shots'].items():
                print(f'  {k}: {v["file"]}  {v["width"]}x{v["height"]} {v["kind"]}  recorded {v["recorded"]}\n'
                      f'      bench at {v.get("commit")}' +
                      (f'; installs {", ".join(v["installs"])}' if v.get('installs') else '') +
                      (f'; scene {v["scene"]}' if v.get('scene') else '') +
                      (f'; session {v["session"]}' if v.get('session') else '') +
                      f'\n      from {v["source"]}')
            return 0
    except BaselineError as e:
        print(f'a2test baseline: {e}', file=sys.stderr)
        return 2
    print(f'a2test baseline: unknown command {sub!r} (record, compare, diff, list, show)', file=sys.stderr)
    return 2
