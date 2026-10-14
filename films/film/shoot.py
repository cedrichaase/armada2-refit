"""Filming: `probe` and `take` (films/README.md, "The process").

Every take is a fresh bench session: started with the shot's scene and the variant's
installs, the setup block run, the action block recorded, the session stopped. Takes run
in parallel, `--jobs` at once."""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from concurrent.futures import ThreadPoolExecutor

from . import storyboard as sb
from .cli import command, load, select, take_state

REPO = sb.REPO
A2TEST = os.path.join(REPO, 'a2test')
SESSIONS = os.path.expanduser('~/.cache/a2test/sessions')
READY_TIMEOUT = 300     # a session's game and scene, from start to built
TAIL = 0.5              # seconds recorded after the action's last line

_print_lock = threading.Lock()


def say(tag, msg):
    with _print_lock:
        print(f'{time.strftime("%H:%M:%S")} {tag:18} {msg}', flush=True)


def probe_duration(path):
    r = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', path],
                       capture_output=True, text=True)
    try:
        return float(r.stdout.strip())
    except ValueError:
        return 0.0


def commit():
    r = subprocess.run(['git', '-C', REPO, 'describe', '--always', '--dirty'], capture_output=True, text=True)
    return r.stdout.strip()


class Take:
    """One shot, one variant, one tier: a session from start to stop."""

    def __init__(self, film, shot, variant, tier):
        self.film, self.shot, self.variant, self.tier = film, shot, variant, tier
        self.stem = film.take(shot, variant, tier)
        self.tag = f'{shot.id}/{variant}'
        self.sid = None
        self.sess = None
        self.log = None

    def note(self, msg):
        say(self.tag, msg)
        if self.log:
            self.log.write(f'{time.strftime("%H:%M:%S")} {msg}\n')
            self.log.flush()

    # ---------------------------------------------------------------- the session
    def start(self):
        f, s = self.film, self.shot
        env = dict(os.environ)
        ini = f.scene_ini(s)
        if ini:
            env['SCENE_INI'] = ini
        if not f.music:
            env['SCENE_MUSIC'] = '0'
        cmd = [A2TEST, 'session', 'start', '--res', f.res(self.tier),
               *f.variants[self.variant].split(), '--args', f.args(s),
               '--label', f'film-{f.name}-{s.id}-{self.variant}-{self.tier}']
        r = subprocess.run(cmd, cwd=REPO, env=env, capture_output=True, text=True)
        self.log.write(r.stdout + r.stderr)
        m = re.search(r'^session (\S+)', r.stdout, re.M)
        if not m:
            raise RuntimeError('session did not start: ' + (r.stdout + r.stderr).strip()[-300:])
        self.sid = m.group(1)
        self.note(f'session {self.sid}')

    def drive(self, *args):
        r = subprocess.run([A2TEST, 'drive', '--session', self.sid, *args], cwd=REPO,
                           capture_output=True, text=True)
        out = (r.stdout + r.stderr).strip()
        self.log.write(f'  drive {" ".join(args)} -> {out[:400]}\n')
        return out

    def wait_ready(self):
        """Until the scene answers `query` with its objects (or, with no scene, the game runs)."""
        t0 = time.time()
        has_scene = self.film.scene_ini(self.shot) is not None
        seen = False     # `running` says "not running" until the game's process is up
        while time.time() - t0 < READY_TIMEOUT:
            if 'not running' in self.drive('running'):
                if seen or time.time() - t0 > 120:
                    raise RuntimeError('the game died before the scene was built')
                time.sleep(3)
                continue
            seen = True
            if not has_scene or 'object' in self.drive('scene', 'query'):
                time.sleep(3)    # the first frames after the build are still settling
                return
            time.sleep(4)
        raise RuntimeError(f'no scene after {READY_TIMEOUT} s')

    def stop(self):
        if self.sid:
            subprocess.run([A2TEST, 'session', 'stop', self.sid], cwd=REPO, capture_output=True, text=True)
            self.note('session stopped')

    # ---------------------------------------------------------------- input at human speed
    def bench(self):
        if self.sess is None:
            sys.path.insert(0, os.path.join(REPO, 'testbench'))
            from bench.session import Session
            self.sess = Session.load(os.path.join(SESSIONS, self.sid, 'session.json'))
        return self.sess

    def ptr(self, x, y, secs=0.6):
        s = self.bench()
        cx, cy = s.s.get('cursor') or [x, y]
        n = max(1, int(secs * 60))
        t0 = time.time()
        for i in range(1, n + 1):
            u = i / n
            e = u * u * (3 - 2 * u)
            s.move(cx + (x - cx) * e, cy + (y - cy) * e)
            dt = t0 + secs * u - time.time()
            if dt > 0:
                time.sleep(dt)

    def run(self, lines, events=None, t0=None):
        for ln, line in lines:
            op, _, arg = line.partition(' ')
            if events is not None:
                events.append(dict(t=round(time.time() - t0, 3), op=op, arg=arg))
            if op == 'wait':
                time.sleep(float(arg))
            elif op == 'scene':
                out = self.drive('scene', *[c.strip() for c in arg.split(';')])
                if re.search(r'unknown|error|no such', out, re.I):
                    self.note(f'line {ln}: scene {arg} -> {out[:200]}')
            elif op in ('key', 'shot', 'step'):
                self.drive(op, arg)
            elif op == 'click':
                self.drive('click', *arg.split())
            elif op == 'ptr':
                self.ptr(*[float(v) for v in arg.split()])
            elif op == 'drag':
                a = [float(v) for v in arg.split()]
                self.ptr(a[0], a[1], 0.4)
                time.sleep(0.15)
                self.bench().input('down 1')
                time.sleep(0.1)
                self.ptr(a[2], a[3], a[4] if len(a) > 4 else 1.0)
                time.sleep(0.2)
                self.bench().input('up 1')
            elif op == 'lclick':
                a = [float(v) for v in arg.split()]
                if a:
                    self.ptr(*a)
                time.sleep(0.12)
                self.bench().input('down 1')
                time.sleep(0.08)
                self.bench().input('up 1')
            elif op == 'press':
                self.bench().key(arg)

    # ---------------------------------------------------------------- recording
    def record_start(self, raw):
        st = json.load(open(os.path.join(SESSIONS, self.sid, 'session.json')))
        e = st['env']
        env = dict(os.environ, WAYLAND_DISPLAY=e['WAYLAND_DISPLAY'], XDG_RUNTIME_DIR=e['XDG_RUNTIME_DIR'])
        # never -r: a fixed rate stamps frames at that rate and drifts from the sound
        # when the game draws fewer (films/README.md, "Traps")
        x264 = ['-p', 'preset=ultrafast', '-p', 'crf=18'] if self.tier == 'probe' else \
               ['-p', 'preset=veryfast', '-p', 'crf=14']
        self.rec = subprocess.Popen(['wf-recorder', '-o', 'HEADLESS-1', '-y', '-c', 'libx264', *x264,
                                     '-f', raw + '.mkv'], env=env, stdin=subprocess.DEVNULL,
                                    stdout=self.log, stderr=subprocess.STDOUT, start_new_session=True)
        sink = (st.get('null_sink') or {}).get('name')
        self.aud = subprocess.Popen(['parec', '-d', sink + '.monitor', '--file-format=wav', raw + '.wav'],
                                    stdin=subprocess.DEVNULL, stdout=self.log, stderr=subprocess.STDOUT,
                                    start_new_session=True) if sink else None
        return time.time()

    def record_stop(self):
        for p in (self.rec, self.aud):
            if p:
                try:
                    os.killpg(p.pid, 2)
                except ProcessLookupError:
                    pass
        for p in (self.rec, self.aud):
            if p:
                try:
                    p.wait(20)
                except subprocess.TimeoutExpired:
                    p.kill()

    def wait_recording(self, raw):
        """Until wf-recorder is writing frames: it takes seconds to deliver its first."""
        t0 = time.time()
        while time.time() - t0 < 15:
            try:
                if os.path.getsize(raw + '.mkv') > 20000:
                    break
            except OSError:
                pass
            time.sleep(0.2)
        time.sleep(0.5)

    def mux(self, raw, duration):
        """raw.mkv + raw.wav -> <stem>.mkv, the action's `duration` seconds and no more.

        Each recording started at its own moment, but both stopped at the same one, so the
        action is the last `duration` seconds of each (less the moment they take to stop).

        wf-recorder writes limited-range pixels flagged full-range: the flag is rewritten
        in the stream first, with no re-encode, so the trim below does not rescale them."""
        fixed = raw + '-tv.mkv'
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', raw + '.mkv', '-c:v', 'copy',
                        '-bsf:v', 'h264_metadata=video_full_range_flag=0', fixed], check=True)
        lead = max(0.0, probe_duration(fixed) - duration)
        has_audio = os.path.exists(raw + '.wav') and os.path.getsize(raw + '.wav') > 1000
        audio_in = ['-ss', f'{max(0.0, probe_duration(raw + ".wav") - duration):.3f}', '-i', raw + '.wav'] \
            if has_audio else ['-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo']
        x264 = ['-preset', 'veryfast', '-crf', '18'] if self.tier == 'probe' else ['-preset', 'slow', '-crf', '14']
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-ss', f'{lead:.3f}', '-i', fixed, *audio_in,
                        '-map', '0:v', '-map', '1:a', '-fps_mode', 'passthrough',
                        '-c:v', 'libx264', *x264, '-pix_fmt', 'yuv420p',
                        '-color_range', 'tv', '-colorspace', 'bt470bg', '-color_primaries', 'bt470bg',
                        '-color_trc', 'smpte170m', '-c:a', 'pcm_s16le', '-ar', '48000', '-ac', '2',
                        '-t', f'{duration:.3f}', self.stem + '.mkv'], check=True)

    def contact_sheet(self, duration):
        """Six frames over the action, labelled with their time."""
        tmp = tempfile.mkdtemp(dir=os.path.dirname(self.stem))
        try:
            n = 6
            frames = []
            for i in range(n):
                t = duration * (i + 0.5) / n
                out = os.path.join(tmp, f'{i}.png')
                subprocess.run(['ffmpeg', '-v', 'error', '-y', '-ss', f'{t:.2f}', '-i', self.stem + '.mkv',
                                '-frames:v', '1', '-vf', 'scale=480:-2', out], check=False)
                if os.path.exists(out):
                    frames += ['-label', f'{t:.1f}s', out]
            if frames:
                subprocess.run(['magick', 'montage', *frames, '-tile', f'{n}x1', '-geometry', '+2+2',
                                '-background', '#222', '-fill', 'white', '-pointsize', '14',
                                '-title', f'{self.shot.id} {self.shot.name} - {self.variant} ({self.tier})',
                                self.stem + '.sheet.png'], check=False)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    # ---------------------------------------------------------------- the whole take
    def __call__(self):
        os.makedirs(os.path.dirname(self.stem), exist_ok=True)
        self.log = open(self.stem + '.log', 'w')
        raw = tempfile.mkdtemp(prefix=f'{self.variant}-', dir=os.path.dirname(self.stem)) + '/raw'
        ok = False
        try:
            self.start()
            self.wait_ready()
            self.run(self.shot.setup)
            self.record_start(raw)
            self.wait_recording(raw)
            t0 = time.time()
            events = []
            self.run(self.shot.action, events, t0)
            time.sleep(TAIL)
            t1 = time.time()
            duration = t1 - t0
            self.record_stop()
            self.stop()
            self.sid = None
            self.mux(raw, duration)
            json.dump(dict(start=t0, duration=round(duration, 3), events=events),
                      open(self.stem + '.events.json', 'w'), indent=1)
            json.dump(dict(hash=self.film.spec_hash(self.shot, self.variant, self.tier),
                           spec=self.film.spec(self.shot, self.variant, self.tier), commit=commit(),
                           res=self.film.res(self.tier), tier=self.tier,
                           filmed=time.strftime('%Y-%m-%d %H:%M:%S')),
                      open(self.stem + '.spec.json', 'w'), indent=1)
            self.contact_sheet(probe_duration(self.stem + '.mkv'))
            self.note(f'done, {duration:.1f} s -> {self.stem}.mkv')
            ok = True
        except Exception as e:      # one failed take must not end the others
            self.note(f'FAILED: {e}')
        finally:
            if getattr(self, 'rec', None):
                self.record_stop()
            self.stop()
            shutil.rmtree(os.path.dirname(raw), ignore_errors=True)
            self.log.close()
        return ok


def film_takes(a, tier):
    film = load(a.board)
    probs = [(s, m) for s, m in sb.check(film) if s is None or not a.shots or s.id in a.shots]
    if probs:
        for s, m in probs:
            print(f'! {s.id if s else "film"}: {m}')
        sys.exit('a2film: the board has problems (./a2film check)')
    todo = []
    for s in select(film, a.shots):
        if not s.filmed:
            continue
        for v in s.variants:
            if a.variant and v not in a.variant:
                continue
            if not a.force and take_state(film, s, v, tier) == 'ok':
                say(f'{s.id}/{v}', 'current, skipped (--force to re-film)')
                continue
            todo.append(Take(film, s, v, tier))
    if not todo:
        print('nothing to film')
        return 0
    say('a2film', f'{len(todo)} takes at {film.res(tier)}, {a.jobs} at once')

    def staggered(i_take):
        i, t = i_take
        time.sleep(min(i, a.jobs - 1) * 8 if i < a.jobs else 0)   # never two starts in one second
        return t()

    with ThreadPoolExecutor(a.jobs) as ex:
        results = list(ex.map(staggered, enumerate(todo)))
    bad = [t.tag for t, ok in zip(todo, results) if not ok]
    say('a2film', f'{len(todo) - len(bad)} of {len(todo)} takes filmed' + (f'; failed: {", ".join(bad)}' if bad else ''))
    return 1 if bad else 0


OPTS = [(('--jobs', '-j'), dict(type=int)),
        (('--force',), dict(action='store_true', help='re-film takes that are current')),
        (('--variant',), dict(action='append', help='only this variant (repeatable)'))]


@command(options=OPTS)
def probe(a):
    """film the shots at the probe resolution, several at once (default 4)"""
    a.jobs = a.jobs or 4
    return film_takes(a, 'probe')


@command(options=OPTS)
def take(a):
    """film the final takes at the final resolution, one at a time by default"""
    a.jobs = a.jobs or 1
    return film_takes(a, 'final')
