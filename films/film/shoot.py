"""Filming: `probe` and `take` (films/README.md, "The process").

Every take is a fresh bench session: started with the shot's scene and the variant's
installs, the setup block run, the action block recorded, the session stopped. Takes run
in parallel, `--jobs` at once."""
import glob
import json
import os
import re
import shlex
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
GOG_APP = '1174788223'  # Armada II's GOG app id, as Heroic records it
DESKTOP_ROWS = 900      # a desktop shot's logical height: a 1600x900 desktop, so a window fills it

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
        self.app = None         # a desktop shot's program, once `run` has started it
        self.desktop = shot.kind == 'desktop'
        # pointer coordinates are written in the film's Pointer space: a game take scales
        # them to its pixels, a desktop take to its logical units (its output is scaled to
        # DESKTOP_ROWS logical rows at every tier, so a window is the same size in each)
        w, h = (int(v) for v in film.res(tier).split('x'))
        pw, ph = film.pointer
        self.scale = h / DESKTOP_ROWS
        self.k = (w / self.scale / pw, DESKTOP_ROWS / ph) if self.desktop else (w / pw, h / ph)

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
        label = f'film-{f.name}-{s.id}-{self.variant}-{self.tier}'
        if self.desktop:      # no game: a stock clone to install into, and its display
            cmd = [A2TEST, 'session', 'start', '--res', f.res(self.tier), '--mod', 'stock', '--no-launch',
                   '--label', label]
        else:
            cmd = [A2TEST, 'session', 'start', '--res', f.res(self.tier),
                   *f.variants[self.variant].split(), '--args', f.args(s), '--label', label]
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
        if self.desktop:
            return self.desktop_prepare()
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
        if getattr(self, 'wall', None) and self.wall.poll() is None:
            self.wall.terminate()
        if self.app and self.app.poll() is None:
            try:
                os.killpg(self.app.pid, 15)
                self.app.wait(10)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                pass
        if self.sid:
            subprocess.run([A2TEST, 'session', 'stop', self.sid], cwd=REPO, capture_output=True, text=True)
            self.note('session stopped')

    # ---------------------------------------------------------------- a desktop, no game
    def swaymsg(self, *cmd):
        r = subprocess.run(['swaymsg', *cmd], env=self.bench().wl_env(), capture_output=True, text=True)
        self.log.write(f'  swaymsg {" ".join(cmd)} -> {(r.stdout + r.stderr).strip()[:300]}\n')

    def wallpaper(self):
        """`Wallpaper: <shot> [variant] [t]`, a still from that shot's take at this tier,
        or `Wallpaper: #rrggbb`. Black without one."""
        w = (self.shot.get('Wallpaper') or '#000000').split()
        if w[0].startswith('#'):
            png = self.stem + '.wallpaper.png'
            subprocess.run(['magick', '-size', '16x9', f'xc:{w[0]}', png], check=True)
            return png
        src = self.film.shot(w[0])
        v = w[1] if len(w) > 1 else src.variants[-1]
        t = float(w[2]) if len(w) > 2 else 1.0
        mkv = self.film.take(src, v, self.tier) + '.mkv'
        if not os.path.exists(mkv):
            raise RuntimeError(f'Wallpaper {w[0]} {v}: no take at this tier yet ({mkv})')
        png = self.stem + '.wallpaper.png'
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-ss', f'{t:.2f}', '-i', mkv, '-frames:v', '1', png],
                       check=True)
        return png

    def desktop_prepare(self):
        """The bare desktop: scaled to 1080 logical rows, the wallpaper, every window
        floating in the middle; and a scratch HOME in which Heroic knows the session's
        stock clone, so an installer finds the game there and installs into it."""
        self.swaymsg('output', 'HEADLESS-1', 'scale', f'{self.scale:.4f}')
        self.swaymsg('for_window [app_id=".*"] floating enable, move position center')
        self.swaymsg('for_window [class=".*"] floating enable, move position center')
        # the wallpaper is a window, tiled under the floating ones: there is no swaybg here
        self.swaymsg('for_window [app_id="a2film-wallpaper"] floating disable')
        self.wall = subprocess.Popen(['mpv', '--wayland-app-id=a2film-wallpaper', '--no-audio', '--no-osc',
                                      '--no-input-default-bindings', '--keep-open=yes', '--image-display-duration=inf',
                                      '--cursor-autohide=no', '--no-border', '--panscan=1.0', self.wallpaper()],
                                     env=self.bench().wl_env(), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                     stderr=subprocess.DEVNULL, start_new_session=True)
        home = os.path.join(self.bench().work, 'home')
        heroic = os.path.join(home, '.config', 'heroic')
        os.makedirs(os.path.join(heroic, 'gog_store'), exist_ok=True)
        os.makedirs(os.path.join(heroic, 'GamesConfig'), exist_ok=True)
        clone = str(self.bench().game_dir)
        # the clone may carry the real install's record of a release: this is a fresh game
        try:
            os.remove(os.path.join(clone, 'armada2-refit-installed.json'))
        except FileNotFoundError:
            pass
        # where Heroic puts it, so the window shows ~/Games/Heroic/Star Trek Armada II: the
        # clone is bind-mounted there inside the program's namespace (run_app), since the
        # installer resolves links and would show the bench's path
        game = os.path.join(home, 'Games', 'Heroic', 'Star Trek Armada II')
        os.makedirs(game, exist_ok=True)
        self.clone, self.game = clone, game
        # the user's own vkBasalt layer, which the installer looks for under XDG_DATA_HOME
        layers = os.path.join(home, '.local', 'share', 'vulkan', 'implicit_layer.d')
        os.makedirs(layers, exist_ok=True)
        for src in glob.glob(os.path.expanduser('~/.local/share/vulkan/implicit_layer.d/vkBasalt*.json')):
            shutil.copy(src, layers)
        json.dump(dict(installed=[dict(appName=GOG_APP, install_path=game, platform='windows')]),
                  open(os.path.join(heroic, 'gog_store', 'installed.json'), 'w'))
        json.dump({GOG_APP: dict(enviromentOptions=[]), 'version': 'v0', 'explicit': True},
                  open(os.path.join(heroic, 'GamesConfig', f'{GOG_APP}.json'), 'w'))
        self.home = home
        self.note(f'desktop at scale {self.scale:.3f}, HOME {home}, the game at {game}')
        time.sleep(2.0)

    def run_app(self):
        """Start the shot's `Run:` program on the desktop: leading NAME=value words are its
        environment, `{repo}` this checkout. It runs in its own PID and mount namespace: it
        sees none of the bench's other games (nor the user's Heroic) among the processes,
        and the session's clone of the game at ~/Games/Heroic."""
        words = shlex.split(self.shot.get('Run').replace('{repo}', REPO))
        env = dict(self.bench().wl_env(), HOME=self.home, XDG_CONFIG_HOME=os.path.join(self.home, '.config'),
                   XDG_CACHE_HOME=os.path.join(self.home, '.cache'),
                   XDG_DATA_HOME=os.path.join(self.home, '.local', 'share'), PATH=os.environ['PATH'],
                   LANG=os.environ.get('LANG', 'C.UTF-8'), DISPLAY=self.bench().s['env']['DISPLAY'])
        # nothing of the user's own desktop session: no bus to reach their Heroic or browser
        for k in ('DBUS_SESSION_BUS_ADDRESS', 'XDG_SESSION_ID', 'XDG_CURRENT_DESKTOP', 'HYPRLAND_INSTANCE_SIGNATURE'):
            env.pop(k, None)
        while words and re.fullmatch(r'[A-Z_][A-Z0-9_]*=.*', words[0]):
            k, _, v = words.pop(0).partition('=')
            env[k] = v
        # root in a namespace of its own to bind-mount, then the user again for the program
        uid, gid = os.getuid(), os.getgid()
        self.app = subprocess.Popen(['unshare', '--user', '--map-root-user', '--mount', '--pid', '--fork',
                                     '--mount-proc', 'sh', '-c',
                                     f'mount --bind "$0" "$1" && shift && exec unshare --user --map-user={uid} '
                                     f'--map-group={gid} "$@"',
                                     self.clone, self.game, *words], env=env, cwd=self.home, stdin=subprocess.DEVNULL,
                                    stdout=self.log, stderr=subprocess.STDOUT, start_new_session=True)
        self.note(f'running {" ".join(words)}')

    # ---------------------------------------------------------------- input at human speed
    def bench(self):
        if self.sess is None:
            sys.path.insert(0, os.path.join(REPO, 'testbench'))
            from bench.session import Session
            self.sess = Session.load(os.path.join(SESSIONS, self.sid, 'session.json'))
        return self.sess

    def xy(self, x, y):
        return x * self.k[0], y * self.k[1]

    def ptr(self, x, y, secs=0.6):
        s = self.bench()
        x, y = self.xy(x, y)
        if self.desktop and s.s.get('cursor') is None:
            # the first move is absolute, over the output's pixels; every later one is
            # relative, in the compositor's logical units (Session.move)
            s.input(f'move {int(x * self.scale)} {int(y * self.scale)}')
            s.s['cursor'] = [int(x), int(y)]
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
                a = arg.split()
                x, y = self.xy(float(a[0]), float(a[1]))
                button = int(a[a.index('--button') + 1]) if '--button' in a else 1
                if self.desktop:
                    self.ptr(float(a[0]), float(a[1]), 0.3)
                    self.bench().input(f'down {button}')
                    time.sleep(0.08)
                    self.bench().input(f'up {button}')
                else:
                    self.drive('click', str(round(x)), str(round(y)), *a[2:])
            elif op == 'run':
                self.run_app()
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
        # a still desktop sends no damage, so no frames: ask for every frame (-D)
        nodamage = ['-D'] if self.desktop else []
        self.rec = subprocess.Popen(['wf-recorder', '-o', 'HEADLESS-1', *nodamage, '-y', '-c', 'libx264', *x264,
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
