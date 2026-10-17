"""./a2film review -- an automatic first pass over the takes (films/README.md, "Review").

For every take of the chosen shots (probe takes by default): a contact sheet of the shot's
In..In+Length window with a margin, a motion curve, and flags with timestamps (black,
frozen, flashes and white squares, a take that ends early, silent or clipping sound);
for a shot with two variants, how they differ over time; one HTML overview per board;
a summary table in the terminal. The footage is decoded once per take, small (256 px wide,
15 frames a second), through ffmpeg into numpy; nothing is filmed or changed.
"""
import html
import json
import os
import subprocess
from concurrent.futures import ThreadPoolExecutor

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from .cli import command, load, select
from .edit import FONT, TIER_OPT, probe_media, speed, take_path

FPS = 15                  # analysis frame rate
AW, AH = 256, 144         # analysis frame size
MAX_SECS = 120            # decode no more than this of a take
MARGIN = 1.0              # seconds shown before In and after In+Length on the sheet
TILES = 12                # frames on a sheet
COLS = 4
BLACK_MEAN, BLACK_FLAT = 6.0, 10     # a black frame: mean luma, and nothing (99.9th percentile) brighter than the mean + this;
#                                      limited-range black reads ~3, and a dark sky with a ship in it is not black
BLACK_MIN = 0.4                      # seconds before black is a flag
FROZEN_DIFF, FROZEN_MIN = 0.12, 1.5  # mean |frame difference| (of 255) and seconds
FLASH_JUMP = 40.0                    # luminance jump up that comes back down within 0.25 s
WHITE_JUMP = 0.004                   # a solid white block this much of the frame, new this frame
SILENT_PEAK, SILENT_MIN = 0.002, 2.0 # peak amplitude (of full scale), seconds
CLIP_SAMPLES = 20                    # samples at full scale in the window
SAME_DIFF, FAR_DIFF = 1.5, 30.0      # variants: mean difference below / above these (of 255)
COLOURS = dict(black='#d04040', frozen='#4a90d9', flash='#e0c030', white='#e0c030', early='#c060c0')


# ------------------------------------------------------------------ decoding
def decode(path, secs):
    """(frames uint8 [n,AH,AW,3], gray uint8 [n,AH,AW]) of the first `secs` seconds at FPS."""
    cmd = ['ffmpeg', '-v', 'error', '-t', f'{secs:.2f}', '-i', path, '-an', '-vf',
           f'fps={FPS},scale={AW}:{AH}:flags=area,format=rgb24', '-f', 'rawvideo', '-']
    raw = subprocess.run(cmd, capture_output=True).stdout
    n = len(raw) // (AW * AH * 3)
    fr = np.frombuffer(raw[:n * AW * AH * 3], np.uint8).reshape(n, AH, AW, 3)
    gray = np.empty((n, AH, AW), np.uint8)
    for i in range(0, n, 64):
        c = fr[i:i + 64].astype(np.float32)
        gray[i:i + 64] = (c[..., 0] * .299 + c[..., 1] * .587 + c[..., 2] * .114).astype(np.uint8)
    return fr, gray


def decode_audio(path, secs):
    cmd = ['ffmpeg', '-v', 'error', '-t', f'{secs:.2f}', '-i', path, '-vn', '-ac', '1', '-ar', '16000',
           '-f', 's16le', '-']
    raw = subprocess.run(cmd, capture_output=True).stdout
    return np.frombuffer(raw[:len(raw) // 2 * 2], np.int16)


def stretches(mask, min_secs, fps=FPS):
    """[(t0, t1)] of runs of True at least `min_secs` long."""
    out, i, n = [], 0, len(mask)
    while i < n:
        if mask[i]:
            j = i
            while j < n and mask[j]:
                j += 1
            if (j - i) / fps >= min_secs:
                out.append((i / fps, j / fps))
            i = j
        else:
            i += 1
    return out


def smooth(x, k):
    if len(x) < k:
        return x
    return np.convolve(x, np.ones(k) / k, mode='same')


# ------------------------------------------------------------------ analysis of one take
def analyse(path, shot, variant):
    """Everything measured on one take, as a dict (the sheet and the table read it)."""
    dur, has_audio = probe_media(path)
    secs = min(dur, MAX_SECS) if dur else MAX_SECS
    fr, gray = decode(path, secs)
    n = len(gray)
    spd = speed(shot)
    i0 = shot.in_point(variant)
    length = shot.length
    w0 = i0
    w1 = i0 + (length * spd if length else max(0.0, dur - i0))
    r = dict(shot=shot.id, variant=variant, path=path, duration=round(dur, 2), frames=n,
             window=[round(w0, 2), round(w1, 2)], flags=[], suggest={})
    if n < 2:
        r['flags'].append(dict(kind='unreadable', t0=0, t1=0, text='no frames decoded'))
        r['luma'], r['motion'] = [], []
        return r, fr, gray

    g = gray.reshape(n, -1)
    luma = g.mean(axis=1)
    p99 = np.percentile(g, 99.9, axis=1)
    motion = np.abs(np.diff(g.astype(np.int16), axis=0)).mean(axis=1)       # [n-1], between frames
    r['luma'] = [round(float(x), 1) for x in luma]
    r['motion'] = [round(float(x), 3) for x in motion]
    flags = r['flags']

    def inside(t0, t1):
        return t1 > w0 and t0 < w1

    def add(kind, t0, t1, text):
        flags.append(dict(kind=kind, t0=round(t0, 2), t1=round(t1, 2), text=text,
                          inside=inside(t0, t1 if t1 > t0 else t0 + 1 / FPS)))

    black = (luma < BLACK_MEAN) & (p99 <= np.maximum(luma + BLACK_FLAT, 0))
    for a, b in stretches(black, BLACK_MIN):
        add('black', a, b, f'black {a:.1f}-{b:.1f}s')
    if shot.kind != 'desktop':
        still = np.concatenate([[False], motion < FROZEN_DIFF]) & ~black
        for a, b in stretches(still, FROZEN_MIN):
            add('frozen', a, b, f'frozen {a:.1f}-{b:.1f}s')
    # luminance spikes: up by FLASH_JUMP and back down within 0.25 s
    d = np.diff(luma)
    k = max(1, int(0.25 * FPS))
    t = 0
    while t < len(d):
        if d[t] > FLASH_JUMP:
            back = [j for j in range(t + 1, min(len(d), t + 1 + k)) if d[j] < -FLASH_JUMP * 0.6]
            if back:
                add('flash', (t + 1) / FPS, (back[0] + 1) / FPS, f'flash at {(t + 1) / FPS:.2f}s '
                    f'(luma +{d[t]:.0f})')
                t = back[0]
        t += 1
    # a new, solid, small-to-medium block of saturated white
    white = (fr.min(axis=3) >= 250)
    frac = white.reshape(n, -1).mean(axis=1)
    last = -99
    for i in range(1, n):
        if frac[i] - frac[i - 1] > WHITE_JUMP and frac[i] < 0.2 and i - last > FPS // 2:
            ys, xs = np.nonzero(white[i] & ~white[i - 1])
            if len(ys) >= 8:
                box = (ys.max() - ys.min() + 1) * (xs.max() - xs.min() + 1)
                if len(ys) / box > 0.6 and box < AW * AH * 0.2:
                    add('white', i / FPS, i / FPS, f'white square at {i / FPS:.2f}s '
                        f'({xs.min() * 100 // AW}%,{ys.min() * 100 // AH}% {100 * len(ys) / (AW * AH):.1f}% of frame)')
                    last = i
    # the take ends before the shot does
    if length and dur < w1 - 0.05:
        flags.append(dict(kind='early', t0=round(dur, 2), t1=round(w1, 2), inside=True,
                          text=f'take ends {w1 - dur:.1f}s before In+Length ({dur:.1f} < {w1:.1f}s)'))
    # sound
    if (shot.get('Audio', 'game').split() or ['game'])[0] == 'game':
        if not has_audio:
            flags.append(dict(kind='silent', t0=0, t1=0, inside=True, text='no audio stream (Audio: game)'))
        else:
            a = decode_audio(path, secs)
            sr = 16000
            lo, hi = int(w0 * sr), min(len(a), int(w1 * sr))
            seg = np.abs(a[lo:hi].astype(np.int32)) if hi > lo else np.zeros(0, np.int32)
            if len(seg):
                r['audio_peak'] = round(float(seg.max()) / 32768, 3)
                if seg.max() / 32768 < SILENT_PEAK:
                    flags.append(dict(kind='silent', t0=round(w0, 2), t1=round(w1, 2), inside=True,
                                      text='silent (Audio: game)'))
                else:
                    blk = sr // 4
                    nb = len(seg) // blk
                    pk = seg[:nb * blk].reshape(nb, blk).max(axis=1) / 32768
                    for s0, s1 in stretches(pk < SILENT_PEAK, SILENT_MIN, fps=4):
                        flags.append(dict(kind='silent', t0=round(w0 + s0, 2), t1=round(w0 + s1, 2), inside=True,
                                          text=f'silent gap {w0 + s0:.1f}-{w0 + s1:.1f}s'))
                    clipped = int((seg >= 32700).sum())
                    if clipped >= CLIP_SAMPLES:
                        where = np.nonzero(seg >= 32700)[0]
                        flags.append(dict(kind='clip', t0=round(w0 + where[0] / sr, 2), t1=round(w0 + where[-1] / sr, 2),
                                          inside=True, text=f'audio clips ({clipped} samples, first at '
                                          f'{w0 + where[0] / sr:.1f}s)'))
    # motion peaks and where it settles
    sm = smooth(motion, max(1, int(0.4 * FPS)))
    peaks, settle = [], None
    if len(sm) > 4:
        top = float(sm.max())
        order = np.argsort(sm)[::-1]
        for i in order:
            if sm[i] < 0.3 * top or len(peaks) >= 3:
                break
            if all(abs(i - p) > FPS for p in peaks):
                peaks.append(int(i))
        peaks.sort()
        if top > 0.5:
            pk = int(np.argmax(sm))
            lim = max(0.2 * top, 0.3)
            hold = int(0.5 * FPS)
            for i in range(pk, len(sm) - hold):
                if (sm[i:i + hold] < lim).all():
                    settle = (i + 1) / FPS
                    break
    r['peaks'] = [round((p + 1) / FPS, 2) for p in peaks]
    r['settle'] = None if settle is None else round(settle, 2)
    r['motion_peak_t'] = None if not peaks else r['peaks'][int(np.argmax([sm[p] for p in peaks]))]

    # suggestions, only where obvious
    sug = r['suggest']
    lead = [f for f in flags if f['kind'] == 'black' and f['t0'] <= 0.1 and f['t1'] > w0]
    if lead:
        sug['In'] = round(lead[0]['t1'] + 0.2, 1)
        sug['why_in'] = f'black until {lead[0]["t1"]:.1f}s'
    elif (settle and r['motion_peak_t'] is not None and r['motion_peak_t'] < w0 + 1.0
          and settle > w0 + 0.3 and settle < w1 and float(sm.max()) > 3 * float(np.median(sm)) + 0.2):
        sug['In'] = round(settle + 0.1, 1)
        sug['why_in'] = f'motion settles at {settle:.1f}s'
    inn = sug.get('In', w0)
    tail = [f for f in flags if f['kind'] in ('black', 'frozen') and inn < f['t0'] < w1 - 0.3]
    if any(f['kind'] == 'early' for f in flags):
        sug['Length'] = round(max(0.0, (dur - inn) / spd - 0.1), 1)
        sug['why_length'] = 'take ends early'
    elif tail:
        sug['Length'] = round((tail[0]['t0'] - inn) / spd, 1)
        sug['why_length'] = f'{tail[0]["kind"]} from {tail[0]["t0"]:.1f}s'
    elif 'In' in sug and length:
        # a later In keeps the length only while the take reaches it
        if dur < inn + length * spd:
            sug['Length'] = round(max(0.0, (dur - inn) / spd - 0.1), 1)
            sug['why_length'] = 'fits the take from the new In'
    return r, fr, gray


# ------------------------------------------------------------------ variants
def compare(a, b, shot):
    """How two variants differ over the window, from their gray frames and In points."""
    (ra, ga), (rb, gb) = a, b
    ia, ib = shot.in_point(ra['variant']), shot.in_point(rb['variant'])
    length = (shot.length or 0) * speed(shot)
    n = int(min(len(ga) - ia * FPS, len(gb) - ib * FPS, (length or 1e9) * FPS))
    if n < 2:
        return None
    sa = ga[int(round(ia * FPS)):][:n, ::4, ::4].astype(np.int16)
    sb = gb[int(round(ib * FPS)):][:n, ::4, ::4].astype(np.int16)
    diff = np.abs(sa - sb).reshape(n, -1).mean(axis=1)
    # histogram distance: half the L1 distance of the normalised 32-bin luma histograms
    hd = []
    for x, y in zip(sa, sb):
        hx = np.histogram(x, 32, (0, 256))[0] / x.size
        hy = np.histogram(y, 32, (0, 256))[0] / y.size
        hd.append(0.5 * np.abs(hx - hy).sum())
    # a constant offset between the two that makes them agree better (camera or timing)
    best, bests = 0, float(diff.mean())
    sg = ga[:, ::4, ::4].astype(np.int16)
    sh = gb[:, ::4, ::4].astype(np.int16)
    for k in range(-FPS, FPS + 1):          # shift variant b's In by k frames
        m = min(len(sg) - int(round(ia * FPS)), len(sh) - (int(round(ib * FPS)) + k), n)
        if int(round(ib * FPS)) + k < 0 or m < FPS:
            continue
        d = np.abs(sg[int(round(ia * FPS)):][:m] - sh[int(round(ib * FPS)) + k:][:m]).mean()
        if d < bests * 0.7 and d < bests - 1.0:
            best, bests = k, float(d)
    out = dict(a=ra['variant'], b=rb['variant'], diff=[round(float(x), 2) for x in diff],
               hist=[round(float(x), 3) for x in hd], mean=round(float(diff.mean()), 2),
               p95=round(float(np.percentile(diff, 95)), 2), hist_mean=round(float(np.mean(hd)), 3),
               flags=[], shift=None)
    lay = shot.layout[0]
    if out['p95'] < SAME_DIFF:
        out['flags'].append(f'{ra["variant"]} and {rb["variant"]} barely differ (mean {out["mean"]}, p95 {out["p95"]})')
    elif out['mean'] > FAR_DIFF:
        t = 'the wipe halves will not line up' if lay in ('wipe', 'sweep') else 'the variants differ everywhere'
        out['flags'].append(f'{t} (mean difference {out["mean"]})')
    if best:
        out['shift'] = round(best / FPS, 2)
        out['flags'].append(f'{rb["variant"]} agrees better {best / FPS:+.2f}s later '
                            f'(mean {out["mean"]} -> {round(bests, 2)}): In: {rb["variant"]} {max(0, ib + best / FPS):.1f}')
    return out


# ------------------------------------------------------------------ pictures
def _font(size):
    try:
        return ImageFont.truetype(FONT, size)
    except OSError:
        return ImageFont.load_default()


def plot(draw, box, series, colour, ymax, label, w0, w1, dur, marks=(), flags=()):
    x0, y0, x1, y1 = box
    draw.rectangle(box, fill='#202020')
    if dur <= 0:
        return
    sx = (x1 - x0) / dur
    draw.rectangle((x0 + w0 * sx, y0, min(x1, x0 + w1 * sx), y1), fill='#2e3a2e')
    for f in flags:
        c = COLOURS.get(f['kind'])
        if c and f['t1'] > f['t0']:
            draw.rectangle((x0 + f['t0'] * sx, y1 - 5, x0 + f['t1'] * sx, y1), fill=c)
        elif c:
            draw.line((x0 + f['t0'] * sx, y0, x0 + f['t0'] * sx, y1), fill=c)
    pts = [(x0 + (i + 1) / FPS * sx, y1 - min(1.0, v / ymax) * (y1 - y0 - 6) - 6) for i, v in enumerate(series)]
    if len(pts) > 1:
        draw.line(pts, fill=colour, width=1)
    for t, c in marks:
        draw.line((x0 + t * sx, y0, x0 + t * sx, y1), fill=c)
    draw.text((x0 + 4, y0 + 2), label, fill='#bbbbbb', font=_font(11))


def sheet_png(res, fr, shot, out):
    dur = res['duration'] or len(fr) / FPS
    w0, w1 = res['window']
    t0 = max(0.0, w0 - MARGIN)
    t1 = min(len(fr) / FPS, max(w1, w0 + 1) + MARGIN)
    rows = (TILES + COLS - 1) // COLS
    pad, head, plot_h = 4, 22, 70
    W = COLS * (AW + pad) + pad
    H = head + rows * (AH + pad) + pad + plot_h * 2 + 30
    im = Image.new('RGB', (W, H), '#181818')
    d = ImageDraw.Draw(im)
    f12, f11 = _font(13), _font(11)
    title = f'{res["shot"]} {res["variant"]}   take {dur:.1f}s   window {w0:.1f}-{w1:.1f}s'
    d.text((pad + 2, 4), title, fill='#F2A33A', font=f12)
    for i in range(TILES):
        t = t0 + (t1 - t0) * (i + 0.5) / TILES
        k = min(len(fr) - 1, int(t * FPS))
        x, y = pad + (i % COLS) * (AW + pad), head + (i // COLS) * (AH + pad)
        tile = Image.fromarray(fr[k])
        if not (w0 <= t <= w1):
            tile = Image.eval(tile, lambda v: v // 2)
        im.paste(tile, (x, y))
        d.rectangle((x, y, x + 44, y + 14), fill='#000000')
        d.text((x + 3, y + 1), f'{k / FPS:.2f}s', fill='#F2A33A' if w0 <= t <= w1 else '#888888', font=f11)
        for fl in res['flags']:
            if fl['t0'] - 0.5 / FPS <= k / FPS <= max(fl['t1'], fl['t0']) + 0.5 / FPS and fl['kind'] in COLOURS:
                d.rectangle((x, y, x + AW - 1, y + AH - 1), outline=COLOURS[fl['kind']], width=2)
    y = head + rows * (AH + pad) + pad
    fl = [f for f in res['flags'] if f['kind'] in COLOURS]
    marks = [(p, '#F2A33A') for p in res['peaks']] + ([(res['settle'], '#60d060')] if res['settle'] else [])
    mx = max([0.5] + list(res['motion']))
    plot(d, (pad, y, W - pad, y + plot_h), res['motion'], '#e0e0e0', mx,
         f'motion (mean frame difference, max {mx:.1f}); peaks amber, settle green; shaded: In..In+Length', w0, w1, dur,
         marks, fl)
    plot(d, (pad, y + plot_h + 4, W - pad, y + 2 * plot_h + 4), res['luma'], '#80b0ff', max(40.0, max(res['luma'])),
         'luminance', w0, w1, dur, (), fl)
    x = pad
    for k, c in COLOURS.items():
        d.rectangle((x, H - 16, x + 8, H - 8), fill=c)
        d.text((x + 11, H - 18), k, fill='#aaaaaa', font=f11)
        x += 70
    im.save(out)


def variants_png(cmp, shot, w0, w1, out):
    n = len(cmp['diff'])
    W, H = 4 * (AW + 4) + 4, 150
    im = Image.new('RGB', (W, H), '#181818')
    d = ImageDraw.Draw(im)
    d.text((6, 4), f'{shot.id}: {cmp["a"]} vs {cmp["b"]} over In..In+Length   mean {cmp["mean"]}   '
           f'histogram distance {cmp["hist_mean"]}', fill='#F2A33A', font=_font(13))
    secs = n / FPS
    mx = max(5.0, max(cmp['diff']))
    plot(d, (4, 24, W - 4, 84), cmp['diff'], '#e0e0e0', mx, f'mean pixel difference (max {mx:.0f}/255)', 0, secs, secs)
    plot(d, (4, 88, W - 4, 144), cmp['hist'], '#80b0ff', 1.0, 'luma histogram distance (0..1)', 0, secs, secs)
    im.save(out)


# ------------------------------------------------------------------ the page and the table
def window_text(r):
    return f'{r["window"][0]:g}-{r["window"][1]:g}'


def suggestion(r):
    s = r['suggest']
    parts = []
    if 'In' in s:
        parts.append(f'In {s["In"]:g}')
    if 'Length' in s:
        parts.append(f'Length {s["Length"]:g}')
    return ', '.join(parts)


def short_flags(r):
    kinds = {}
    for f in r['flags']:
        if f.get('inside', True):
            kinds.setdefault(f['kind'], []).append(f)
    out = []
    for k, fs in kinds.items():
        if k in ('black', 'frozen'):
            out.append(f'{k} ' + ','.join(f'{f["t0"]:.1f}-{f["t1"]:.1f}' for f in fs[:3]))
        elif k in ('flash', 'white'):
            out.append(f'{k} ' + ','.join(f'{f["t0"]:.1f}' for f in fs[:4]) + ('+' if len(fs) > 4 else ''))
        elif k == 'early':
            out.append(f'ends {fs[0]["t1"] - fs[0]["t0"]:.1f}s early')
        elif k == 'clip':
            out.append(f'clips {fs[0]["t0"]:.1f}s')
        elif k == 'silent':
            out.append('silent' if fs[0]['text'].startswith('silent (') else f'silent gap {fs[0]["t0"]:.1f}-{fs[0]["t1"]:.1f}')
        else:
            out.append(fs[0]['text'])
    return '; '.join(out)


def write_html(film, tier, results, path):
    css = ('body{background:#141414;color:#ddd;font:14px sans-serif;margin:16px}h1{color:#F2A33A;font-size:20px}'
           'h2{font-size:16px;margin:24px 0 4px;color:#F2A33A}table{border-collapse:collapse;margin:8px 0}'
           'td,th{border:1px solid #333;padding:3px 8px;text-align:left;vertical-align:top}th{background:#222}'
           'img{max-width:100%;display:block;margin:6px 0}.flag{color:#ff8a70}.ok{color:#8c8}.sug{color:#9fd49f}'
           'ul{margin:4px 0;padding-left:18px}a{color:#8cf}')
    out = [f'<!doctype html><meta charset="utf-8"><title>{html.escape(film.title)} review</title><style>{css}</style>',
           f'<h1>{html.escape(film.title)} &mdash; review of the {tier} takes</h1>',
           '<table><tr><th>shot</th><th>variant</th><th>window (s)</th><th>flags</th><th>suggested</th><th>settle</th></tr>']
    for sid, rs in results.items():
        for r in rs['takes']:
            fl = short_flags(r)
            out.append(f'<tr><td><a href="#{sid}">{sid}</a></td><td>{r["variant"]}</td><td>{window_text(r)}</td>'
                       f'<td class="{"flag" if fl else "ok"}">{html.escape(fl or "none")}</td>'
                       f'<td class="sug">{html.escape(suggestion(r))}</td><td>{r["settle"] if r["settle"] else ""}</td></tr>')
        if rs.get('compare'):
            out.append(f'<tr><td>{sid}</td><td colspan=5>{html.escape(rs["compare"]["a"])} vs {html.escape(rs["compare"]["b"])}: '
                       f'mean difference {rs["compare"]["mean"]}'
                       + ''.join(f'<br><span class="flag">{html.escape(x)}</span>' for x in rs['compare']['flags']) + '</td></tr>')
    out.append('</table>')
    base = os.path.dirname(path)
    for sid, rs in results.items():
        out.append(f'<h2 id="{sid}">{sid} &middot; {html.escape(rs["name"])}</h2>')
        for r in rs['takes']:
            out.append(f'<div><b>{r["variant"]}</b> &mdash; take {r["duration"]} s, window {window_text(r)} s'
                       + (f', audio peak {r["audio_peak"]}' if 'audio_peak' in r else '') + '<ul>'
                       + ''.join(f'<li class="{"flag" if f.get("inside", True) else ""}">{html.escape(f["text"])}'
                                 f'{"" if f.get("inside", True) else " (outside the window)"}</li>' for f in r['flags'])
                       + (f'<li>motion peaks at {", ".join(str(p) for p in r["peaks"])} s'
                          + (f', settles at {r["settle"]} s' if r['settle'] else '') + '</li>')
                       + (f'<li class="sug">suggested: {html.escape(suggestion(r))} ({html.escape(", ".join(v for k, v in r["suggest"].items() if k.startswith("why")))})</li>'
                          if r['suggest'] else '') + '</ul>')
            out.append(f'<img src="{os.path.relpath(r["sheet"], base)}" loading="lazy"></div>')
        if rs.get('compare_png'):
            out.append(f'<img src="{os.path.relpath(rs["compare_png"], base)}" loading="lazy">')
    with open(path, 'w') as f:
        f.write('\n'.join(out))


@command(options=[TIER_OPT, (('--jobs',), dict(type=int, default=4, help='takes analysed at once (default 4)'))])
def review(a):
    """a first pass over the takes: contact sheets, flags, variant differences, motion curves, an HTML overview"""
    film = load(a.board)
    shots = [s for s in select(film, a.shots) if s.filmed]
    jobs = []
    for s in shots:
        for v in s.variants:
            p = take_path(film, s, v, a.tier)
            if p:
                jobs.append((s, v, p))
    if not jobs:
        print(f'review: no {a.tier} takes for those shots')
        return 1

    def work(j):
        s, v, p = j
        r, fr, gray = analyse(p, s, v)
        r['sheet'] = film.take(s, v, a.tier) + '.review.png'
        if len(fr) >= 2:
            sheet_png(r, fr, s, r['sheet'])
        return r, gray

    with ThreadPoolExecutor(max(1, a.jobs)) as pool:
        done = list(pool.map(work, jobs))
    results = {}
    for (s, v, p), (r, gray) in zip(jobs, done):
        rs = results.setdefault(s.id, dict(name=s.name, takes=[], grays=[]))
        rs['takes'].append(r)
        rs['grays'].append((r, gray))
    for s in shots:
        rs = results.get(s.id)
        if not rs:
            continue
        if len(rs['grays']) >= 2 and not s.players and s.kind != 'desktop':
            c = compare(rs['grays'][0], rs['grays'][1], s)
            if c:
                rs['compare'] = c
                rs['compare_png'] = os.path.join(film.take_dir(s, a.tier), 'variants.review.png')
                variants_png(c, s, 0, 0, rs['compare_png'])
        del rs['grays']

    rows = []
    for s in shots:
        rs = results.get(s.id)
        if not rs:
            continue
        for r in rs['takes']:
            rows.append((s.id, r['variant'], window_text(r), short_flags(r) or '-', suggestion(r) or '-'))
        c = rs.get('compare')
        if c:
            rows.append((s.id, f'{c["a"]}/{c["b"]}', f'diff {c["mean"]}', '; '.join(c['flags']) or '-', '-'))
    heads = ('shot', 'variant', 'window', 'flags', 'suggested')
    wd = [max(len(str(r[i])) for r in rows + [heads]) for i in range(5)]
    wd[3] = min(wd[3], 70)
    print('  '.join(h.ljust(w) for h, w in zip(heads, wd)).rstrip())
    for r in rows:
        print('  '.join(str(c).ljust(w) for c, w in zip(r, wd)).rstrip())

    page = os.path.join(film.data_dir(), f'review-{a.tier}.html')
    os.makedirs(film.data_dir(), exist_ok=True)
    write_html(film, a.tier, results, page)
    with open(os.path.join(film.data_dir(), f'review-{a.tier}.json'), 'w') as f:
        json.dump({sid: {k: v for k, v in rs.items() if k != 'compare_png'} for sid, rs in results.items()}, f)
    nflag = sum(1 for r in rows if r[3] != '-')
    print(f'\nreview: {len(jobs)} takes, {nflag} rows with flags\noverview: {page}')
    return 0
