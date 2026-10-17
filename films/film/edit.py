"""Cutting: the contact sheet, the animatic and the manifest (films/README.md).

The animatic is a rough cut, made from the storyboard's edit direction alone, so the
whole film can be watched while it is still being directed. Each shot is rendered to a
segment of its own (constant frame rate, the tier's size, 48 kHz stereo PCM, BT.709
limited range), and the segments are joined with their transitions and laid over the
music beds. The manifest is the same edit as data, for a real editor."""
import json
import os
import shlex
import subprocess
import tempfile
from concurrent.futures import ThreadPoolExecutor

from . import marks
from . import storyboard as sb
from .cli import IGNORE_CODE, command, load, select, take_check
from .storyboard import a2env

TIER_OPT = (('--tier',), dict(choices=('probe', 'final'), default='probe',
                              help='which takes to cut from (default: probe)'))
ACCENT = '0xF2A33A'          # the showcase's LCARS amber
GAME_GAIN = 6.0              # dB on a take's own sound: the game is ~14 dB quieter than the music
VO_GAIN = 10.0
KEY_SECS = 1.2               # how long an automatic key hint stays up


def run(args, quiet=False):
    if not quiet:
        print('  ffmpeg', ' '.join(shlex.quote(a) for a in args)[:200], flush=True)
    subprocess.run(['ffmpeg', '-v', 'error', '-y', *args], check=True)


def font(bold=True):
    # this ffmpeg has no fontconfig, so drawtext needs a file
    r = subprocess.run(['fc-match', '-f', '%{file}', 'sans:bold' if bold else 'sans'],
                       capture_output=True, text=True)
    return r.stdout.strip()


FONT, FONT2 = font(True), font(False)


def probe_media(path):
    """(duration, has_audio) of a file, (0, False) if it cannot be read."""
    r = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format=duration:stream=codec_type',
                        '-of', 'json', path], capture_output=True, text=True)
    try:
        j = json.loads(r.stdout)
    except ValueError:
        return 0.0, False
    d = float(j.get('format', {}).get('duration') or 0)
    return d, any(s.get('codec_type') == 'audio' for s in j.get('streams', []))


def size(film, tier):
    w, h = film.res(tier).lower().split('x')
    return int(w), int(h)


def take_path(film, shot, variant, tier):
    p = film.take(shot, variant, tier) + '.mkv'
    return p if os.path.exists(p) else None


def events(film, shot, variant, tier):
    try:
        return json.load(open(film.take(shot, variant, tier) + '.events.json')).get('events', [])
    except (OSError, ValueError):
        return []


def speed(shot):
    return float(shot.get('Speed', 1) or 1)


# the tier the command runs at: a footage shot's `From:` follows it
TIER = 'probe'


def footage_path(shot):
    return shot.film.footage(shot, TIER)


def seg_len(shot):
    """Seconds on screen. Footage without a Length runs to the end of its file."""
    if shot.length:
        return shot.length
    if shot.kind == 'footage':
        p = footage_path(shot)
        d, _ = probe_media(p) if p and os.path.exists(p) else (0, False)
        return max(0.5, (d - shot.in_point('')) / speed(shot)) if d else 3.0
    return 3.0


def timeline(film, shots=None):
    """[(shot, start, end, overlap)] in the cut's seconds: a dissolve overlaps the shot before."""
    out, t = [], 0.0
    for s in shots if shots is not None else film.shots:
        kind, d = s.transition
        ov = 0.0
        if kind == 'dissolve' and out and d > 0:
            ov = min(d, seg_len(s) / 2, (out[-1][2] - out[-1][1]) / 2)
            t -= ov
        out.append((s, t, t + seg_len(s), ov))
        t += seg_len(s)
    return out


def section_of(film, shot):
    for sec in film.sections:
        if shot in sec.shots:
            return sec


def parse_music(v):
    """`<file> [from S] [gain dB] [until <shot>] [fade S]`, or `none`."""
    if not v:
        return None
    a = v.split()
    if a[0].lower() == 'none':
        return dict(none=True)
    m = dict(file=a[0], **{'from': 0.0}, gain=0.0, until=None, fade=1.5)
    i = 1
    while i + 1 < len(a):
        k, val = a[i].lower(), a[i + 1]
        if k == 'until':
            m['until'] = val
        elif k in ('from', 'gain', 'fade'):
            m[k] = float(val.lstrip('+'))
        i += 2
    return m


def music_file(film, name):
    """A bed's file: a path (absolute or beside the board), else under the game's
    sounds/music, matched without case (the game mixes .wav and .WAV)."""
    for p in (name, os.path.join(film.dir, name)):
        if os.path.isabs(p) and os.path.exists(p):
            return p
    if os.path.exists(os.path.join(film.dir, name)):
        return os.path.join(film.dir, name)
    d = os.path.join(a2env.GAME, 'sounds', 'music')
    try:
        for f in os.listdir(d):
            if f.lower() == name.lower():
                return os.path.join(d, f)
    except OSError:
        pass
    return None


# ------------------------------------------------------------------ drawing
class Graph:
    """A filter graph under construction, with its text files (drawtext reads text from a
    file, so nothing in a title has to survive filter-graph escaping)."""

    def __init__(self, tmp, W, H, fps):
        self.tmp, self.W, self.H, self.fps = tmp, W, H, fps
        self.k = H / 1080          # text and boxes are laid out at 1080 and scaled
        self.parts = []
        self.n = 0

    def textfile(self, text):
        self.n += 1
        p = os.path.join(self.tmp, f't{self.n}.txt')
        with open(p, 'w') as f:
            f.write(text)
        return p

    def text(self, t, x, y, px, color='white', alpha='1', bold=True, enable=None, shadow=True):
        s = (f"drawtext=fontfile={FONT if bold else FONT2}:textfile={self.textfile(t)}:expansion=none"
             f":x={x}:y={y}:fontsize={max(8, int(px * self.k))}:fontcolor={color}:alpha='{alpha}'")
        if shadow:
            s += f':shadowcolor=black@0.7:shadowx={max(1, int(2 * self.k))}:shadowy={max(1, int(2 * self.k))}'
        if enable:
            s += f":enable='{enable}'"
        return s

    def add(self, s):
        self.parts.append(s)

    def write(self):
        p = os.path.join(self.tmp, 'graph.txt')
        with open(p, 'w') as f:
            f.write(';\n'.join(self.parts))
        return p


def fade_alpha(t0, t1, f=0.4):
    return f"if(lt(t,{t0}),0,if(lt(t,{t0}+{f}),(t-{t0})/{f},if(lt(t,{t1}-{f}),1,if(lt(t,{t1}),({t1}-t)/{f},0))))"


def ease(t0, t1, a, b):
    u = f"((t-{t0})/({t1}-{t0}))"
    return f"if(lt(t,{t0}),{a},if(gt(t,{t1}),{b},{a}+({b}-{a})*{u}*{u}*(3-2*{u})))"


def wipe_pos(dur, mid, end):
    """0: all the first variant, 1: all the second. Holds the first briefly, eases to the
    middle, holds the split, then completes (the October showcase's wipe)."""
    t1 = min(2.0, dur * 0.3)
    t2, t3 = max(t1, dur - 2.4), max(t1, dur - 1.0)
    return f"if(lt(t,{dur / 2}),{ease(min(0.6, t1 / 2), t1, 0, mid)},{ease(t2, t3, mid, end)})"


def caption(g, title, sub, t0, t1, top):
    k = g.k
    y = int(120 * k) if top else g.H - int(190 * k)
    a = fade_alpha(t0, t1, 0.5)
    s = (f"drawbox=x={int(60 * k)}:y={y}:w={max(2, int(10 * k))}:h={int(120 * k)}:color={ACCENT}:t=fill"
         f":enable='between(t,{t0},{t1})',"
         + g.text(title.upper(), int(90 * k), y - int(12 * k), 78, 'white', a))
    if sub:
        s += ',' + g.text(sub.upper(), int(92 * k), y + int(74 * k), 40, '0xE6E6E6', a, bold=False)
    return s


def keycaps(g, hints, top):
    """hints: [(t0, t1, text)], an amber keycap centred between the caption and the action."""
    out = []
    y = int((330 if top else 150) * g.k)
    for t0, t1, txt in hints:
        txt = txt.upper()
        w = int((34 * len(txt) + 60) * g.k)
        out.append(f"drawbox=x={(g.W - w) // 2}:y={y}:w={w}:h={int(92 * g.k)}:color={ACCENT}@0.92:t=fill"
                   f":enable='between(t,{t0},{t1})'")
        out.append(g.text(txt, '(w-tw)/2', y + int(12 * g.k), 64, '0x161616', '1',
                          enable=f'between(t,{t0},{t1})', shadow=False))
    return ','.join(out)


def key_hints(film, shot, tier, variant, dur):
    keys = (shot.get('Keys') or 'auto').strip()
    if keys == 'none':
        return []
    sp, i0 = speed(shot), shot.in_point(variant)
    out = []
    if keys == 'auto':
        for e in events(film, shot, variant, tier):
            if e.get('op') == 'press':
                t = (float(e['t']) - i0) / sp
                if 0 <= t < dur:
                    out.append((round(t, 3), round(min(dur, t + KEY_SECS), 3), str(e.get('arg', ''))))
        return out
    for part in keys.split(';'):
        a = part.strip().split(None, 1)
        if len(a) == 2:
            t = float(a[0]) / sp
            out.append((t, min(dur, t + KEY_SECS), a[1]))
    return out


def label_color(variant):
    return 'white' if variant == 'stock' else ACCENT


# ------------------------------------------------------------------ segments
def render_card(film, shot, tier, g, inputs, dur):
    W, H = g.W, g.H
    bg = shot.get('Background')
    src = None
    if bg:
        try:
            b = film.shot(bg)
            for v in reversed(b.variants):
                src = take_path(film, b, v, tier)
                if src:
                    inputs += ['-ss', str(b.in_point(v)), '-t', str(dur + 0.5), '-i', src]
                    break
        except sb.BoardError:
            pass
    if src:
        g.add(f"[0:v]setpts=PTS-STARTPTS,{fit(W, H, film.fps, True)},gblur=sigma={18 * g.k},"
              f"eq=brightness=-0.18,trim=duration={dur}[bg]")
    else:
        inputs += ['-f', 'lavfi', '-t', str(dur), '-i', f'color=black:s={W}x{H}:r={film.fps}']
        g.add(f"[0:v]format=yuv420p[bg]")
    style = dict(big=(150, 'white', True), heading=(72, 'white', True), accent=(40, ACCENT, True),
                 small=(34, '0xBBBBBB', False))
    lines = [(style.get(st, style['heading']), t) for st, t in shot.card]
    # the whole card shrinks, keeping its proportions, until its widest line fits 88% of
    # the width; drawtext cannot measure, so the width is estimated at 0.58 em a letter
    widest = max((len(t) * px * 0.58 for (px, _, _), t in lines), default=0)
    k = min(1.0, 1690 / widest) if widest else 1.0
    lines = [((int(px * k), col, bold), t) for (px, col, bold), t in lines]
    total = sum(int(px * 1.45) for (px, _, _), _ in lines)
    y = (1080 - total) // 2
    chain = []
    for i, ((px, col, bold), t) in enumerate(lines):
        t0 = 0.3 + 0.3 * i
        chain.append(g.text(t, '(w-tw)/2', int(y * g.k), px, col, fade_alpha(t0, dur - 0.2, 0.5), bold, shadow=False))
        y += int(px * 1.45)
    g.add('[bg]' + (','.join(chain) or 'null') + '[c]')
    return '[c]', None


# a take is captured BT.601 limited range; the cut is BT.709 limited range
CS = 'colorspace=all=bt709:iall=bt601-6-625:irange=tv:range=tv:fast=1'


def fit(W, H, fps, take):
    """Any input to the frame, converted to the cut's colours if it is a take."""
    cs = CS + ',' if take else ''
    return (f"{cs}scale={W}:{H}:force_original_aspect_ratio=decrease:flags=lanczos,"
            f"pad={W}:{H}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps={fps},format=yuv420p")


def render_shot(film, shot, tier, g, inputs, dur):
    """The layout of a filmed (or footage) shot. Returns (video label, audio input index or None)."""
    W, H, fps = g.W, g.H, film.fps
    sp = speed(shot)
    crop = shot.crop
    if shot.kind == 'footage':
        variants = ['footage']
        paths = {'footage': footage_path(shot) if footage_path(shot) and os.path.exists(footage_path(shot)) else None}
    else:
        variants = shot.variants
        paths = {v: take_path(film, shot, v, tier) for v in variants}
    lay, args = shot.layout
    if lay == 'full':
        named = [x for x in args if x in variants]
        variants = [named[0] if named else variants[-1]]
    elif lay == 'pip':
        corner_v = args[0] if args and args[0] in variants else variants[-1]
        variants = [v for v in variants if v != corner_v][:1] + [corner_v]
    else:
        variants = variants[:2]
    idx, audio = {}, None
    for v in variants:
        p = paths.get(v)
        i0 = shot.in_point('' if shot.kind == 'footage' else v)
        idx[v] = len([a for a in inputs if a == '-i'])
        if p:
            inputs += ['-ss', str(i0), '-t', str(dur * sp + 0.5), '-i', p]
            _, has_a = probe_media(p)
            if has_a and (audio is None or v == variants[-1]):
                audio = idx[v]
            pre = f"[{idx[v]}:v]{CS},setpts=(PTS-STARTPTS)/{sp},"
            if crop:
                x, y, w, h = crop
                pre += f"crop=iw*{w}:ih*{h}:iw*{x}:ih*{y},"
            pre += fit(W, H, fps, False)
        else:
            # not filmed yet: a slate, so the animatic always builds
            inputs += ['-f', 'lavfi', '-t', str(dur), '-i', f'color=0x3a3a3a:s={W}x{H}:r={fps}']
            pre = (f"[{idx[v]}:v]format=yuv420p,"
                   + g.text(f'{shot.id} · {shot.name}', '(w-tw)/2', '(h/2)-th', 56, 'white')
                   + ',' + g.text(f'{v}: not filmed', '(w-tw)/2', '(h/2)+20', 40, '0xBBBBBB', bold=False))
        g.add(pre + f",trim=duration={dur},setpts=PTS-STARTPTS[v_{idx[v]}]")
    L = [f'[v_{idx[v]}]' for v in variants]
    lab_y = int(40 * g.k)
    if lay == 'full' or len(variants) < 2:
        chain = ''
        if 'label' in args:
            v = variants[0]
            chain = g.text(v.upper(), 'w-tw-48' if v == 'stock' else int(48 * g.k), lab_y, 54, label_color(v))
        g.add(f"{L[0]}{chain or 'null'}[lay]")
    elif lay == 'wipe':
        nums = [float(x) for x in args if _num(x)]
        mid, end = (nums + [0.5, 1.0])[:2] if nums else (0.5, 1.0)
        p = wipe_pos(dur, mid, end)
        dw = max(2, int(4 * g.k))
        # the first variant sits left of the divider, masked by a white plate whose right edge
        # rides it; overlay re-evaluates x per frame (blend expressions are far slower)
        g.add(f"color=black:s={W}x{H}:r={fps}:d={dur}[m0];color=white:s={W}x{H}:r={fps}:d={dur}[m1];"
              f"[m0][m1]overlay=x='{W}*(1-({p}))-{W}':y=0:eval=frame,format=gray[mask];"
              f"{L[0]}format=yuva420p[a0];[a0][mask]alphamerge[am];"
              f"color=white:s={dw}x{H}:r={fps}:d={dur}[div];"
              f"{L[1]}[am]overlay=0:0[w0];[w0][div]overlay=x='{W}*(1-({p}))-{dw // 2}':y=0:eval=frame,"
              + g.text(variants[0].upper(), int(48 * g.k), lab_y, 54, label_color(variants[0]),
                       f"min(1,max(0,(1-({p}))*6-0.2))") + ','
              + g.text(variants[1].upper(), 'w-tw-48', lab_y, 54, label_color(variants[1]),
                       f"min(1,max(0,({p})*6-0.2))") + '[lay]')
    elif lay == 'dissolve':
        nums = [float(x) for x in args if _num(x)]
        hold, fade = (nums + [2.2, 1.4])[:2] if len(nums) >= 2 else ((nums[0], 1.4) if nums else (2.2, 1.4))
        if hold + fade > dur:
            hold, fade = dur * 0.35, dur * 0.3
        t1 = hold + fade
        ls = f"if(lt(t,{hold}),1,if(lt(t,{t1}),({t1}-t)/{fade},0))"
        lr = f"if(lt(t,{hold}),0,if(lt(t,{t1}),(t-{hold})/{fade},1))"
        g.add(f"{L[0]}trim=duration={t1}[d0];{L[1]}trim=duration={dur - hold},setpts=PTS-STARTPTS[d1];"
              f"[d0][d1]xfade=transition=fade:duration={fade}:offset={hold},"
              + g.text(variants[0].upper(), int(48 * g.k), lab_y, 54, label_color(variants[0]), ls) + ','
              + g.text(variants[1].upper(), int(48 * g.k), lab_y, 54, label_color(variants[1]), lr) + '[lay]')
    elif lay == 'split':
        # `split [w]`: each variant's centre w of its width (0.5), fitted into half the frame;
        # 0.75 is a 4:3 menu whole, letterboxed
        nums = [float(x) for x in args if _num(x)]
        cw = min(1.0, max(0.1, nums[0])) if nums else 0.5
        half = (f"crop=iw*{cw}:ih:iw*{(1 - cw) / 2}:0,scale={W // 2}:{H}:force_original_aspect_ratio=decrease,"
                f"pad={W // 2}:{H}:(ow-iw)/2:(oh-ih)/2:black,setsar=1")
        g.add(f"{L[0]}{half}[s0];{L[1]}{half}[s1];[s0][s1]hstack,"
              f"drawbox=x={W // 2 - 1}:y=0:w=2:h={H}:color=white:t=fill,"
              + g.text(variants[0].upper(), int(48 * g.k), lab_y, 54, label_color(variants[0])) + ','
              + g.text(variants[1].upper(), 'w-tw-48', lab_y, 54, label_color(variants[1])) + '[lay]')
    elif lay == 'pip':
        corner = next((x for x in args if x in ('tl', 'tr', 'bl', 'br')), 'br')
        m = int(40 * g.k)
        x = m if corner[1] == 'l' else f'W-w-{m}'
        y = m if corner[0] == 't' else f'H-h-{m}'
        g.add(f"{L[1]}scale={W // 3}:-2,pad=iw+4:ih+4:2:2:white,"
              + g.text(variants[1].upper(), 16, 12, 34, label_color(variants[1])) + "[pp];"
              f"{L[0]}[pp]overlay=x={x}:y={y}[lay]")
    return '[lay]', audio


def _num(x):
    try:
        float(x)
        return True
    except ValueError:
        return False


def render_segment(film, shot, tier, out, fade_out, tmp):
    """One shot of the animatic, to `out` (mkv: H.264 BT.709 limited range, PCM)."""
    W, H = size(film, tier)
    dur = seg_len(shot)
    g = Graph(tmp, W, H, film.fps)
    inputs = []
    if shot.kind == 'card':
        v, audio = render_card(film, shot, tier, g, inputs, dur)
        audio = None
        main_variant = None
    else:
        v, audio = render_shot(film, shot, tier, g, inputs, dur)
        main_variant = shot.variants[-1] if shot.kind != 'footage' else ''
    chain = []
    top = (shot.get('Caption') or '').strip().lower() == 'top'
    if main_variant is not None:
        hints = key_hints(film, shot, tier, main_variant, dur) if shot.kind != 'footage' else []
        if hints:
            chain.append(keycaps(g, hints, top))
    if shot.get('Title'):
        chain.append(caption(g, shot.get('Title'), shot.get('Sub'), 0.4, dur - 0.3, top))
    kind, d = shot.transition
    if kind == 'fade' and d:
        chain.append(f'fade=t=in:st=0:d={d}')
    elif kind == 'dip' and d:
        chain.append(f'fade=t=in:st=0:d={d / 2}')
    if fade_out:
        chain.append(f'fade=t=out:st={dur - fade_out}:d={fade_out}')
    chain.append('format=yuv420p')
    g.add(f"{v}{','.join(chain)}[vout]")
    # sound: the take's own, or silence
    mode = (shot.get('Audio') or 'game').strip().lower()
    n_in = len([a for a in inputs if a == '-i'])
    if audio is None or mode == 'none':
        inputs += ['-f', 'lavfi', '-t', str(dur), '-i', 'anullsrc=r=48000:cl=stereo']
        g.add(f"[{n_in}:a]atrim=duration={dur}[aout]")
    else:
        tempo, sp = '', speed(shot)
        while sp > 1.001:
            tempo += f",atempo={min(sp, 2.0)}"
            sp /= min(sp, 2.0)
        while sp < 0.999:
            tempo += f",atempo={max(sp, 0.5)}"
            sp /= max(sp, 0.5)
        gain = VO_GAIN if mode == 'vo' else GAME_GAIN
        g.add(f"[{audio}:a]aresample=48000,aformat=channel_layouts=stereo{tempo},volume={gain}dB,"
              f"apad,atrim=duration={dur},afade=t=in:d=0.03,afade=t=out:st={max(0, dur - 0.05)}:d=0.05[aout]")
    run([*inputs, '-/filter_complex', g.write(), '-map', '[vout]', '-map', '[aout]', '-t', str(dur),
         '-c:v', 'libx264', '-preset', 'veryfast' if tier == 'probe' else 'medium',
         '-crf', '18' if tier == 'probe' else '15', '-pix_fmt', 'yuv420p', '-r', str(film.fps),
         *TAGS, '-c:a', 'pcm_s16le', '-ar', '48000', '-ac', '2', out])


TAGS = ['-colorspace', 'bt709', '-color_primaries', 'bt709', '-color_trc', 'bt709', '-color_range', 'tv']


def assemble(film, tier, tl, segs, out, tmp):
    """Join the segments with their transitions and lay the music beds under them."""
    g = Graph(tmp, *size(film, tier), film.fps)
    inputs = []
    for p in segs:
        inputs += ['-i', p]
    for i in range(len(segs)):
        # xfade wants both its inputs on one timebase: every input, and every concat's
        # output below, is put on 1/fps
        g.add(f"[{i}:v]setpts=PTS-STARTPTS,fps={film.fps},settb=1/{film.fps}[v{i}];[{i}:a]asetpts=PTS-STARTPTS[a{i}]")
    cv, ca = '[v0]', '[a0]'
    for i in range(1, len(segs)):
        shot, start, end, ov = tl[i]
        if ov > 0:
            g.add(f"{cv}[v{i}]xfade=transition=fade:duration={ov}:offset={start}[cv{i}];"
                  f"{ca}[a{i}]acrossfade=d={ov}[ca{i}]")
        else:
            g.add(f"{cv}{ca}[v{i}][a{i}]concat=n=2:v=1:a=1[cc{i}][ca{i}];[cc{i}]settb=1/{film.fps}[cv{i}]")
        cv, ca = f'[cv{i}]', f'[ca{i}]'
    total = tl[-1][2]
    beds = music_beds(film, tl)
    mix = [ca]
    for j, b in enumerate(beds):
        n = len(segs) + j
        inputs += ['-ss', str(b['from']), '-t', str(b['end'] - b['start'] + 0.5), '-i', b['path']]
        L = b['end'] - b['start']
        g.add(f"[{n}:a]aresample=48000,aformat=channel_layouts=stereo,atrim=duration={L},"
              f"afade=t=in:d=0.5,afade=t=out:st={max(0, L - b['fade'])}:d={b['fade']},volume={b['gain']}dB,"
              f"adelay={int(b['start'] * 1000)}:all=1[m{j}]")
        mix.append(f'[m{j}]')
    if len(mix) > 1:
        g.add(f"{''.join(mix)}amix=inputs={len(mix)}:normalize=0:duration=first,"
              f"loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000[aout]")
    else:
        g.add(f"{ca}loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000[aout]")
    g.add(f"{cv}format=yuv420p[vout]")
    run([*inputs, '-/filter_complex', g.write(), '-map', '[vout]', '-map', '[aout]', '-t', str(total),
         '-c:v', 'libx264', '-preset', 'medium', '-crf', '20' if tier == 'probe' else '16',
         '-pix_fmt', 'yuv420p', '-r', str(film.fps), *TAGS, '-c:a', 'aac', '-b:a', '192k',
         '-movflags', '+faststart', out])


def music_beds(film, tl):
    """Each section's bed placed on the cut: from its first shot to `until` (or its last
    shot's end), its fade-out ending there."""
    starts = {s.id: (a, b) for s, a, b, _ in tl}
    beds = []
    for sec in film.sections:
        m = parse_music(sec.fields.get('Music'))
        shots = [s for s in sec.shots if s.id in starts]
        if not m or m.get('none') or not shots:
            continue
        path = music_file(film, m['file'])
        if not path:
            print(f'  music {m["file"]}: not found, left out')
            continue
        start = starts[shots[0].id][0]
        end = starts[m['until']][0] if m['until'] in starts else starts[shots[-1].id][1]
        if end <= start:
            continue
        beds.append(dict(path=path, start=start, end=end, gain=m['gain'], fade=min(m['fade'], end - start),
                         **{'from': m['from']}))
    return beds


@command(options=[TIER_OPT])
def animatic(a):
    """a rough cut from the takes: layouts, titles, key hints, transitions, music"""
    film = load(a.board)
    global TIER
    TIER = a.tier
    shots = select(film, a.shots)
    tl = timeline(film, shots)
    work = os.path.join(film.data_dir(), 'work', a.tier)
    os.makedirs(work, exist_ok=True)
    segs = []
    with tempfile.TemporaryDirectory(prefix='a2film-') as tmp:
        for i, (s, start, end, _) in enumerate(tl):
            nxt = tl[i + 1][0] if i + 1 < len(tl) else None
            fo = 0.0
            if nxt is not None and nxt.transition[0] == 'dip':
                fo = nxt.transition[1] / 2
            elif nxt is None:
                fo = min(0.5, (end - start) / 2)
            out = os.path.join(work, f'{s.id}.mkv')
            print(f'{s.id}: {s.kind}, {end - start:.1f} s', flush=True)
            d = os.path.join(tmp, s.id)
            os.makedirs(d)
            render_segment(film, s, a.tier, out, fo, d)
            segs.append(out)
        name = 'animatic-' + a.tier + ('' if not a.shots else '-' + '-'.join(a.shots)) + '.mp4'
        out = os.path.join(film.data_dir(), name)
        assemble(film, a.tier, tl, segs, out, tmp)
    print(f'animatic: {out} ({tl[-1][2]:.1f} s)')
    return 0


# ------------------------------------------------------------------ the contact sheet
SHEET_FRAMES = 6
TILE_W = 320


def magick(*args):
    subprocess.run(['magick', *args], check=True)


def tile_text(path, w, h, lines, bg='0x262626'):
    """A tile of text lines: [(text, pointsize, colour)]."""
    args = ['-size', f'{w}x{h}', f'xc:#{bg[2:]}']
    y = 12
    for t, pt, col in lines:
        c = '#' + col[2:] if col.startswith('0x') else col
        args += ['-font', FONT, '-pointsize', str(pt), '-fill', c, '-annotate', f'+12+{y + pt}', t]
        y += int(pt * 1.4)
    magick(*args, path)


def grab(src, t, out, w, label=None):
    if src:
        try:
            run(['-ss', f'{max(0, t):.3f}', '-i', src, '-frames:v', '1', '-vf', f'scale={w}:-2', out], quiet=True)
        except subprocess.CalledProcessError:
            src = None
    if not src or not os.path.exists(out):
        magick('-size', f'{w}x{w * 9 // 16}', 'xc:#3a3a3a', '-font', FONT, '-pointsize', '18', '-fill', '#bbbbbb',
               '-gravity', 'center', '-annotate', '+0+0', 'not filmed', out)
    if label:
        magick(out, '-font', FONT, '-pointsize', '16', '-fill', '#F2A33A', '-undercolor', '#000000a0',
               '-gravity', 'northwest', '-annotate', '+6+4', label, out)


@command(options=[TIER_OPT])
def sheet(a):
    """one contact sheet of the film: a row per shot (and variant), in timeline order"""
    film = load(a.board)
    global TIER
    TIER = a.tier
    shots = select(film, a.shots)
    W, H = size(film, a.tier)
    th = TILE_W * H // W
    th -= th % 2
    rows = []
    with tempfile.TemporaryDirectory(prefix='a2film-') as tmp, ThreadPoolExecutor(8) as pool:
        jobs = []
        for s, start, end, _ in timeline(film, shots):
            lay = ' '.join([s.layout[0]] + s.layout[1])
            head = os.path.join(tmp, f'{s.id}-head.png')
            tile_text(head, TILE_W, th, [(s.id, 26, 'white'), (s.name[:28], 16, '0xDDDDDD'),
                                         (f'{s.kind} · {lay}', 15, '0xF2A33A'),
                                         (f'{s.status} · {seg_len(s):g} s · at {start:.1f}', 15, '0xBBBBBB')])
            if s.kind == 'card':
                body = os.path.join(tmp, f'{s.id}-card.png')
                tile_text(body, TILE_W * SHEET_FRAMES, th, [(t, 22 if st in ('big', 'heading') else 16,
                                                            ACCENT if st == 'accent' else 'white')
                                                           for st, t in s.card], bg='0x000000')
                rows.append([head, body])
                continue
            variants = s.variants if s.kind != 'footage' else ['footage']
            for j, v in enumerate(variants):
                src = take_path(film, s, v, a.tier) if s.kind != 'footage' else (
                    footage_path(s) if footage_path(s) and os.path.exists(footage_path(s)) else None)
                i0 = s.in_point(v if s.kind != 'footage' else '')
                span = seg_len(s) * speed(s)
                frames = []
                for f in range(SHEET_FRAMES):
                    t = i0 + span * (f + 0.5) / SHEET_FRAMES
                    out = os.path.join(tmp, f'{s.id}-{v}-{f}.png')
                    frames.append(out)
                    jobs.append(pool.submit(grab, src, t, out, TILE_W,
                                            f'{v} {t - i0:.1f}s' if f == 0 else f'{t - i0:.1f}s'))
                blank = os.path.join(tmp, f'{s.id}-blank.png')
                if j and not os.path.exists(blank):
                    magick('-size', f'{TILE_W}x{th}', 'xc:#262626', blank)
                rows.append([head if j == 0 else blank] + frames)
        for j in jobs:
            j.result()
        row_files = []
        for i, r in enumerate(rows):
            p = os.path.join(tmp, f'row{i}.png')
            magick(*r, '-resize', f'x{th}', '-background', '#181818', '-splice', '4x0', '+append', p)
            row_files.append(p)
        out = os.path.join(film.data_dir(), f'sheet-{a.tier}.png')
        os.makedirs(film.data_dir(), exist_ok=True)
        magick('-background', '#181818', *row_files, '-splice', '0x4', '-append', out)
    print(f'sheet: {out}')
    return 0


# ------------------------------------------------------------------ the manifest
@command(shots=False, options=[TIER_OPT, IGNORE_CODE])
def manifest(a):
    """the edit as JSON, with every take, for an editor"""
    film = load(a.board)
    global TIER
    TIER = a.tier
    tl = timeline(film)
    shots = []
    for s, start, end, ov in tl:
        takes = {}
        if s.filmed:
            for v in s.variants:
                stem = film.take(s, v, a.tier)
                path = stem + '.mkv'
                t = dict(path=path if os.path.exists(path) else None, current=False)
                if t['path']:
                    t['duration'], t['audio'] = probe_media(path)
                    try:
                        spec = json.load(open(stem + '.spec.json'))
                    except (OSError, ValueError):
                        spec = {}
                    st, why = take_check(film, s, v, a.tier, a.ignore_code)
                    t.update(commit=spec.get('commit'), dirty=spec.get('dirty'), hash=spec.get('hash'),
                             res=spec.get('res'), current=st == 'ok', stale=why)
                    if os.path.exists(stem + '.events.json'):
                        t['events'] = events(film, s, v, a.tier)
                    doc = marks.load(film, s, v, a.tier)
                    if doc:
                        t['marks'] = doc['marks']
                        if s.get('Cut'):
                            try:
                                i, n = marks.cut(doc, s.get('Cut'), speed(s))
                                t['cut'] = dict(**{'in': round(i, 3)}, length=round(n, 3))
                            except marks.MarkError as e:
                                t['cut'] = dict(error=str(e))
                        if s.get('Expect'):
                            r = marks.verdict(film, s, v, a.tier)
                            t['expect'] = [dict(expect=e, ok=ok, detail=d) for e, ok, d in r[1]] if r else None
                t['in'] = s.in_point(v)
                takes[v] = t
        elif s.kind == 'footage':
            p = footage_path(s)
            takes['footage'] = dict(path=p if p and os.path.exists(p) else None, **{'in': s.in_point('')})
        lay, largs = s.layout
        tr, td = s.transition
        shots.append(dict(
            id=s.id, name=s.name, section=s.section, kind=s.kind, status=s.status, shows=s.shows,
            start=round(start, 3), end=round(end, 3), length=seg_len(s), speed=speed(s),
            layout=dict(type=lay, args=largs), crop=s.crop,
            transition=dict(type=tr, seconds=td, overlap=round(ov, 3)),
            title=s.get('Title'), sub=s.get('Sub'), caption=(s.get('Caption') or 'bottom').lower(),
            audio=(s.get('Audio') or 'game').lower(), keys=s.get('Keys') or 'auto',
            card=[dict(style=st, text=t) for st, t in s.card] or None,
            background=s.get('Background'),
            fields=s.fields, direction='\n'.join(s.prose).strip(),
            setup=[t for _, t in s.setup], action=[t for _, t in s.action], takes=takes))
    secs = []
    for sec in film.sections:
        m = parse_music(sec.fields.get('Music'))
        if m and not m.get('none'):
            m['path'] = music_file(film, m['file'])
        secs.append(dict(title=sec.title, fields=sec.fields, music=m, direction='\n'.join(sec.prose).strip(),
                         shots=[s.id for s in sec.shots]))
    W, H = size(film, a.tier)
    doc = {
        'format': 'a2film-manifest/1',
        'about': 'The edit of an a2film storyboard (films/README.md). Times are seconds of the cut; '
                 'a take\'s "in" is seconds into its file, whose time 0 is the start of the shot\'s action. '
                 'Takes are variable frame rate H.264, BT.601 limited range; the cut is BT.709 limited range. '
                 'A take\'s "marks" are events on that same clock (films/README.md, "Marks").',
        'film': dict(title=film.title, name=film.name, board=os.path.abspath(film.path), fields=film.fields,
                     variants=film.variants, direction='\n'.join(film.prose).strip(),
                     tier=a.tier, width=W, height=H, fps=film.fps, length=round(tl[-1][2], 3) if tl else 0),
        'sections': secs,
        'shots': shots,
        'music': music_beds(film, tl),
    }
    os.makedirs(film.data_dir(), exist_ok=True)
    out = os.path.join(film.data_dir(), 'manifest.json')
    with open(out, 'w') as f:
        json.dump(doc, f, indent=1, ensure_ascii=False)
    print(f'manifest: {out} ({len(shots)} shots, {doc["film"]["length"]} s)')
    return 0
