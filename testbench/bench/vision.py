"""What the bench can measure off a screenshot, without asking a model.

    ocr()             tesseract's words and boxes
    find_text()       a phrase on screen, tolerant of OCR's usual misreads
    measure_stretch() how much wider than it should be something is drawn, against
                      the same thing in a 4:3 reference shot
    frame_diff()      how much two shots differ -- "did anything happen"

THE STRETCH MEASUREMENT

Every layer of this project that fixes an aspect ratio keeps the vertical axis as the
stock game draws it and corrects the horizontal one (hud/README.md: the canvas height
stays 1200; font/README.md: "the vertical axis is already correct and is not touched";
menus: fill the height).  So between a 4:3 reference and a shot at another resolution,
anything drawn right has scaled by exactly H_test / H_ref in BOTH axes.  The
measurement fixes the vertical factor there and searches the horizontal one: it resizes
a patch of the reference by (sy * a, sy) for a range of `a`, template-matches each
against the test shot, and reports the best `a`.  a = 1.00 is undistorted; stock's HUD
at 21:9 measures ~1.79 (hud/README.md's 2.15 / 1.20).
"""
import difflib
import re
import subprocess
from pathlib import Path

import cv2
import numpy as np


# ---------------------------------------------------------------------- OCR

def _passes(gray):
    """The game's text comes in three kinds that no single preprocessing reads: bright
    text on space (the briefing, the HUD numbers) reads best as it is; grey-on-grey
    shell buttons (the in-mission Options) only after an adaptive threshold; mixed
    screens (campaign selection) best with the background subtracted.  Measured on
    bench screenshots -- see testbench/README.md, "OCR"."""
    yield 'gray', gray
    bg = cv2.GaussianBlur(gray, (0, 0), 15)
    yield 'tophat', 255 - cv2.normalize(cv2.subtract(gray, bg), None, 0, 255, cv2.NORM_MINMAX)
    yield 'adaptive', 255 - cv2.adaptiveThreshold(gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C,
                                                  cv2.THRESH_BINARY, 31, -10)


def ocr(path, region=None, scale=None, fast=False):
    """Words on screen: [{text, conf, x, y, w, h, line}] in screen pixels.

    Game text is small; tesseract reads it better at >= ~24 px cap height, so shots
    are upscaled first (and boxes mapped back).  Every preprocessing pass contributes
    its words; `line` is namespaced by pass, so a phrase is matched within one pass's
    reading and never stitched from two.  fast=True runs the first two passes only."""
    img = cv2.imread(str(path))
    if img is None:
        raise ValueError(f'cannot read {path}')
    ox = oy = 0
    if region:
        x, y, w, h = region
        img = img[y:y + h, x:x + w]
        ox, oy = x, y
    if scale is None:
        # measured: at 1440 the shell's text is already big enough, and a 1.5x
        # upscale loses "Graphics Settings" that 1.0x reads at 91% confidence
        # ... and at 800x600 (the stock baseline) 2x misses the shell's 8 px button
        # text that 3x and 4x read at 90% ("Werewolf Pack", 2026-09-26)
        scale = 4.0 if img.shape[0] <= 700 else 2.0 if img.shape[0] <= 1200 else 1.0
    big = cv2.resize(img, None, fx=scale, fy=scale, interpolation=cv2.INTER_CUBIC)
    gray = cv2.cvtColor(big, cv2.COLOR_BGR2GRAY)
    procs = []
    for name, im in _passes(gray):
        if fast and name == 'adaptive':
            break
        ok, png = cv2.imencode('.png', im)
        procs.append((name, subprocess.Popen(['tesseract', 'stdin', 'stdout', '--psm', '11', 'tsv'],
                                             stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                             stderr=subprocess.DEVNULL), png.tobytes()))
    words = []
    for name, p, data in procs:
        out, _ = p.communicate(data)
        for row in out.decode(errors='replace').splitlines()[1:]:
            f = row.split('\t')
            if len(f) < 12 or not f[11].strip():
                continue
            conf = float(f[10])
            if conf < 30:
                continue
            words.append(dict(text=f[11].strip(), conf=round(conf, 1),
                              x=int(int(f[6]) / scale) + ox, y=int(int(f[7]) / scale) + oy,
                              w=max(1, int(int(f[8]) / scale)), h=max(1, int(int(f[9]) / scale)),
                              line=(name, int(f[2]), int(f[3]), int(f[4])), src=name))
    return words


def dedupe(words):
    """One reading per place, the most confident, for display."""
    keep = []
    for w in sorted(words, key=lambda w: -w['conf']):
        cx, cy = w['x'] + w['w'] / 2, w['y'] + w['h'] / 2
        if not any(k['x'] <= cx <= k['x'] + k['w'] and k['y'] <= cy <= k['y'] + k['h'] for k in keep):
            keep.append(w)
    return keep


def _norm(s):
    return re.sub(r'[^A-Z0-9]', '', s.upper())


def lines(words):
    """Words grouped into tesseract's lines, left to right."""
    out = {}
    for w in words:
        out.setdefault(w['line'], []).append(w)
    return [sorted(v, key=lambda w: w['x']) for v in out.values()]


def union(ws):
    x0 = min(w['x'] for w in ws)
    y0 = min(w['y'] for w in ws)
    x1 = max(w['x'] + w['w'] for w in ws)
    y1 = max(w['y'] + w['h'] for w in ws)
    return dict(x=x0, y=y0, w=x1 - x0, h=y1 - y0, text=' '.join(w['text'] for w in ws),
                conf=round(sum(w['conf'] for w in ws) / len(ws), 1))


def retry_scale(path):
    """The scale to try again at when a phrase is not found, or None.  Frames over
    1200 high are read at 1x (ocr()), which at 3440x1440 read the mission list's
    "Werewolf Pack" as "B er If Pack", twice in a row, where 2x reads it cleanly
    (2026-09-26).  The 1x default stays -- 1.5x once lost "Graphics Settings" at 1440
    -- so 2x is a second look, never the first."""
    img = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE)
    return 2.0 if img is not None and img.shape[0] > 1200 else None


def find_text_in(path, phrase, words=None):
    """find_text on `path`, with a second look at retry_scale() if the first misses."""
    box = find_text(words if words is not None else ocr(path), phrase)
    if box is None and (sc := retry_scale(path)):
        box = find_text(ocr(path, scale=sc), phrase)
    return box


def find_text(words, phrase, threshold=0.8):
    """Best match for `phrase` among runs of consecutive words on one line.  Returns
    a box dict with a `score`, or None.  Case and punctuation are ignored, and OCR's
    usual confusions are tolerated through a similarity ratio rather than equality."""
    target = _norm(phrase)
    if not target:
        return None
    n = len(phrase.split())
    best = None
    for ln in lines(words):
        for i in range(len(ln)):
            for j in range(i + 1, min(len(ln), i + n + 2) + 1):
                cand = _norm(''.join(w['text'] for w in ln[i:j]))
                if not cand:
                    continue
                score = difflib.SequenceMatcher(None, target, cand).ratio()
                if best is None or score > best['score']:
                    best = dict(union(ln[i:j]), score=round(score, 3))
    if best and best['score'] >= threshold:
        return best
    return None


# ---------------------------------------------------------------------- pictures

def size(path):
    img = cv2.imread(str(path), cv2.IMREAD_UNCHANGED)
    return img.shape[1], img.shape[0]


def frame_diff(a, b):
    """Mean absolute difference, 0..255."""
    ia, ib = cv2.imread(str(a)), cv2.imread(str(b))
    if ia is None or ib is None or ia.shape != ib.shape:
        return None
    return float(np.mean(cv2.absdiff(ia, ib)))


def mean_luma(path, region=None):
    img = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE)
    if region:
        x, y, w, h = region
        img = img[y:y + h, x:x + w]
    return float(img.mean())


def content_box(path, threshold=4):
    """The bounding box of everything that is not a black bar -- where the picture is.

    A bar is a column (or row) with no signal at all: its brightest pixel is at most
    `threshold`/255.  Dark art is not a bar: the main menu's plate fades to near-black
    space at 21:9, which a "mostly dark" test counted as 267 px of pillarbox, but its
    stars and noise lift every column's maximum well above 4."""
    img = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE)
    cols = np.where(img.max(axis=0) > threshold)[0]
    rows = np.where(img.max(axis=1) > threshold)[0]
    if not len(cols) or not len(rows):
        return None
    return int(cols[0]), int(rows[0]), int(cols[-1] - cols[0] + 1), int(rows[-1] - rows[0] + 1)


def flat_areas(path, region=None, block=32, max_std=1.0, min_mean=20):
    """Untextured, non-black blocks: what a missing texture or an unpainted surface
    looks like.  Space is near-black (mean < 20) and every textured surface in this game
    has noise well above std 1; a flat mid-grey block has neither.  Returns
    dict(fraction, boxes) -- `boxes` merges adjacent flat blocks into rectangles."""
    img = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE).astype(np.float32)
    ox = oy = 0
    if region:
        x, y, w, h = region
        img = img[y:y + h, x:x + w]
        ox, oy = x, y
    H, W = img.shape
    gh, gw = H // block, W // block
    if not gh or not gw:
        return dict(fraction=0.0, boxes=[])
    g = img[:gh * block, :gw * block].reshape(gh, block, gw, block)
    std = g.std(axis=(1, 3))
    mean = g.mean(axis=(1, 3))
    flat = ((std < max_std) & (mean > min_mean)).astype(np.uint8)
    n, labels, stats, _ = cv2.connectedComponentsWithStats(flat, connectivity=4)
    boxes = []
    for i in range(1, n):
        x, y, w, h, area = stats[i]
        if area >= 4:
            boxes.append(dict(x=int(x * block + ox), y=int(y * block + oy), w=int(w * block), h=int(h * block),
                              blocks=int(area)))
    boxes.sort(key=lambda b: -b['blocks'])
    return dict(fraction=round(float(flat.mean()), 4), boxes=boxes)


def measure_stretch(ref_path, test_path, ref_region=None, lo=0.5, hi=2.4):
    """Horizontal stretch of the test shot relative to the reference, for the patch
    `ref_region` (x, y, w, h in reference pixels; default: the middle 50% of the frame).

    Returns dict(stretch, score, sy, at) -- stretch 1.0 = same shape as the reference;
    score is the normalised correlation of the best match (below ~0.5, don't trust it)."""
    ref = cv2.imread(str(ref_path), cv2.IMREAD_GRAYSCALE)
    test = cv2.imread(str(test_path), cv2.IMREAD_GRAYSCALE)
    rh, rw = ref.shape
    th, tw = test.shape
    if ref_region is None:
        ref_region = (rw // 4, rh // 4, rw // 2, rh // 2)
    x, y, w, h = ref_region
    patch = ref[y:y + h, x:x + w]
    sy = th / rh
    # work at a size where the test frame is ~1000 px wide: fast, and still ~0.5% resolution
    # ... but never so small that a short patch (a line of text) drops under ~20 px
    d = min(1.0, max(1000.0 / tw, 20.0 / (h * sy)))
    test_d = cv2.resize(test, None, fx=d, fy=d, interpolation=cv2.INTER_AREA)

    def score(a):
        tw_ = int(round(w * sy * a * d))
        th_ = int(round(h * sy * d))
        if tw_ < 8 or th_ < 8 or tw_ > test_d.shape[1] or th_ > test_d.shape[0]:
            return -1.0, None
        t = cv2.resize(patch, (tw_, th_), interpolation=cv2.INTER_AREA)
        if float(t.std()) < 1.0:
            return -1.0, None
        r = cv2.matchTemplate(test_d, t, cv2.TM_CCOEFF_NORMED)
        _, mx, _, loc = cv2.minMaxLoc(r)
        return float(mx), loc

    best = (-1.0, None, 1.0)
    for a in np.arange(lo, hi, 0.02):
        s, loc = score(a)
        if s > best[0]:
            best = (s, loc, float(a))
    for a in np.arange(best[2] - 0.02, best[2] + 0.02, 0.002):
        s, loc = score(a)
        if s > best[0]:
            best = (s, loc, float(a))
    s, loc, a = best
    at = None if loc is None else (int(loc[0] / d), int(loc[1] / d))
    return dict(stretch=round(a, 3), score=round(s, 3), sy=round(sy, 4), at=at,
                ref_region=list(ref_region))


def text_shape(box):
    return box['w'] / max(1, box['h'])


def annotate(path, out, boxes, color=(0, 255, 0)):
    """A copy of `path` with boxes drawn on it, for the report."""
    img = cv2.imread(str(path))
    for b in boxes:
        x, y, w, h = int(b['x']), int(b['y']), int(b['w']), int(b['h'])
        cv2.rectangle(img, (x, y), (x + w, y + h), color, 2)
        if b.get('label'):
            cv2.putText(img, b['label'], (x, max(12, y - 6)), cv2.FONT_HERSHEY_SIMPLEX, 0.6, color, 2)
    cv2.imwrite(str(out), img)
    return Path(out)
