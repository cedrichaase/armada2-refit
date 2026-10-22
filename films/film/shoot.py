"""Filming: `probe` and `take` (films/README.md, "The process").

Every take is a fresh bench session: started with the shot's scene and the variant's
installs, the setup block run, the action block recorded, the session stopped. Takes run
in parallel, `--jobs` at once."""
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

from . import code
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), 'testbench'))
from bench import slots  # noqa: E402  (the machine-wide queue for game instances)
from . import marks
from . import storyboard as sb
from .cli import IGNORE_CODE, command, load, select, take_check

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


class GameDied(RuntimeError):
    """The game (or its start) failed the way a loaded machine makes it fail: a take is tried again once."""


class Take:
    """One shot, one variant, one tier: a session from start to stop."""

    def __init__(self, film, shot, variant, tier):
        self.film, self.shot, self.variant, self.tier = film, shot, variant, tier
        self.stem = film.take(shot, variant, tier)
        self.tag = f'{shot.id}/{variant}'
        self.sid = None
        self.sess = None
        self.log = None
        self.slot = None        # a game slot taken for it (an Ensemble takes its players' together)
        self.app = None         # a desktop shot's program, once `run` has started it
        self.desktop = shot.kind == 'desktop'
        self.tail = None        # UI.log, followed from the setup on (marks.Tail)
        self.scene_marks = None  # Scene.marks, read before the session (and its clone) goes
        self.suspect = []       # the expectations (Expect:) the take did not meet
        # pointer coordinates are written in the film's Pointer space: a game take scales
        # them to its pixels, a desktop take to its logical units (its output is scaled to
        # DESKTOP_ROWS logical rows at every tier, so a window is the same size in each)
        w, h = (int(v) for v in film.res(tier, shot).split('x'))
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
        self.code = code.record(f, s, self.variant)     # the code it installs, as it starts
        env = dict(os.environ)
        ini = f.scene_ini(s)
        if ini:
            env['SCENE_INI'] = ini
        if not f.music:
            env['SCENE_MUSIC'] = '0'
        label = f'film-{f.name}-{s.id}-{self.variant}-{self.tier}'
        if self.desktop:      # no game: a stock clone to install into, and its display
            cmd = [A2TEST, 'session', 'start', '--res', f.res(self.tier, s), '--mod', 'stock', '--no-launch',
                   '--label', label]
        else:
            # a shot with Prepare: starts without the game, prepares the clone, then launches
            prep = ['--no-launch'] if s.get('Prepare') else [f'--args={f.args(s)}']   # '-nointro' alone reads as a flag
            cmd = [A2TEST, 'session', 'start', '--res', f.res(self.tier, s),
                   *f.variants[s.install_variant(self.variant)].split(), *prep, '--label', label]
            # the sound: categories muted in the clone, and those recorded as stems
            if s.mute:
                cmd += ['--mute', ','.join(s.mute)]
            if s.stems:
                cmd += ['--stems', ','.join(s.stems)]
        # The slot is handed to the session start, whose sway and game hold it from there
        # on; our copy of the descriptor is dropped as soon as it returns.
        slot = self.slot or slots.acquire(self.tag, say=lambda m: self.note(m.splitlines()[0]))[0]
        self.slot = None
        try:
            env.update(slot.env())
            r = subprocess.run(cmd, cwd=REPO, env=env, capture_output=True, text=True, pass_fds=(slot.fd,))
        finally:
            slot.close()
        self.log.write(r.stdout + r.stderr)
        m = re.search(r'^session (\S+)', r.stdout, re.M)
        if not m:
            out = (r.stdout + r.stderr).strip()
            err = ('session did not start: ' + out[-300:])
            raise (GameDied if re.search(r'launcher exited|did not appear', out) else RuntimeError)(err)
        self.sid = m.group(1)
        self.note(f'session {self.sid}')
        if s.get('Prepare'):
            words = s.get('Prepare').split()
            m = re.search(r'^\s*state:\s*(\S+)', r.stdout, re.M)
            state = m.group(1) if m else os.path.join(SESSIONS, self.sid, 'session.json')
            r = subprocess.run([os.path.join(REPO, words[0]), state, *words[1:]], cwd=REPO,
                               capture_output=True, text=True)
            self.log.write(f'$ Prepare: {s.get("Prepare")}\n{r.stdout}{r.stderr}')
            if r.returncode:
                raise RuntimeError(f'Prepare: {s.get("Prepare")} failed (exit {r.returncode})')
            # a launch the bench's audio guard stops (a stream that reached a real output
            # before it was moved) has the game stopped: once more, then give up
            for attempt in (1, 2):
                out = self.drive('launch', *f.args(s).split())
                if out.startswith('launched'):
                    break
                self.note(f'launch {attempt} failed: {out[:200]}')
                if attempt == 2:
                    raise GameDied(f'the game did not launch: {out[:200]}')
                time.sleep(3)
            self.note('prepared and launched')

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
        ini = self.film.scene_ini(self.shot)
        # a scene file with Enable=0 (a mission, the menus) builds no scene to query
        has_scene = ini is not None and not re.search(r'^\s*Enable\s*=\s*0', open(ini).read(), re.M)
        seen = False     # `running` says "not running" until the game's process is up
        while time.time() - t0 < READY_TIMEOUT:
            if 'not running' in self.drive('running'):
                if seen or time.time() - t0 > 120:
                    raise GameDied('the game died before the scene was built')
                time.sleep(3)
                continue
            seen = True
            if not has_scene or 'object' in self.drive('scene', 'query'):
                time.sleep(3)    # the first frames after the build are still settling
                return
            time.sleep(4)
        raise RuntimeError(f'no scene after {READY_TIMEOUT} s')

    def check_alive(self):
        """A game that died under the take (a crash, or killed under load) has nothing
        to record: GameDied, so the take is tried again."""
        if not self.desktop and self.sid and 'not running' in self.drive('running'):
            raise GameDied('the game died during the take')

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
            self.run_line(ln, op, arg)
            if events is not None:
                events[-1]['t1'] = round(time.time() - t0, 3)   # when it was done (a mark)

    def run_line(self, ln, op, arg):
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
        elif op == 'typelog':
            self.typelog(*arg.split(None, 2))
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

    def typelog(self, player, name, marker):
        """Type the word that follows `marker` in a log of another player's game, from its
        last line that has it (the host's join code, from its Online.log), as the bench's
        `Type what follows ... in ... of <player>` does. Waits up to 30 s for it."""
        ens = getattr(self, 'ensemble', None)
        if not ens or player not in ens.takes:
            raise RuntimeError(f'typelog: no player {player} in this shot')
        other = ens.takes[player].bench()
        rx = re.compile(re.escape(marker.strip('"\'')) + r'\W*([A-Za-z0-9-]+)', re.I)
        t0 = time.time()
        while True:
            found = rx.findall(other.game_log(name) or '')
            if found:
                break
            if time.time() - t0 > 30:
                raise RuntimeError(f'typelog: no "{marker}" in {player}\'s {name} after 30 s')
            time.sleep(0.5)
        self.note(f'typelog: {found[-1]} from {player}\'s {name}')
        self.drive('type', found[-1])

    # ---------------------------------------------------------------- marks
    def follow(self):
        """Start following the game's UI.log, for the menu marks (none on a desktop)."""
        if not self.desktop:
            self.tail = marks.Tail(str(self.bench().game_dir / 'UI.log')).start()

    def gather(self):
        """Read what the game wrote for the marks, while its clone is still there."""
        if self.tail:
            self.tail.stop()
        if not self.desktop and self.sid:
            p = self.bench().game_dir / 'Scene.marks'
            self.scene_marks = p.read_text(errors='replace') if p.exists() else None

    def write_marks(self, t0, duration, events):
        """<stem>.marks.json, and the shot's Expect: checked against it."""
        ui = marks.ui_marks(self.tail.lines) if self.tail and self.tail.seen else None
        scene = marks.parse_scene(self.scene_marks) if self.scene_marks else None
        doc = marks.build(t0, duration, scene=scene, ui=ui, events=events)
        exp = self.shot.get('Expect')
        if exp:
            try:
                res = marks.verify(doc, exp)
            except marks.MarkError as e:
                res = [(exp, False, str(e))]
            doc['expect'] = [dict(expect=e, ok=ok, detail=d) for e, ok, d in res]
            self.suspect = [f'{e} ({d})' for e, ok, d in res if ok is False]
            for e, ok, d in res:
                self.note(f'expect {"ok" if ok else "FAILED" if ok is False else "not checked"}: {e} ({d})')
        json.dump(doc, open(self.stem + '.marks.json', 'w'), indent=1)
        n = len(doc['marks'])
        self.note(f'{n} marks' + (f'; SUSPECT, expected {"; ".join(self.suspect)}' if self.suspect else ''))

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
        # each stem's own sink (Stems: ...); what stays on the session's sink is the effects
        self.stem_recs = {}
        for cat, ss in sorted((st.get('stem_sinks') or {}).items()):
            self.stem_recs[cat] = subprocess.Popen(
                ['parec', '-d', ss['name'] + '.monitor', '--file-format=wav', f'{raw}.{cat}.wav'],
                stdin=subprocess.DEVNULL, stdout=self.log, stderr=subprocess.STDOUT, start_new_session=True)
        if self.stem_recs and self.shot.stems and set(self.stem_recs) != set(self.shot.stems):
            self.note(f'stems: the session has {", ".join(self.stem_recs)}, the shot wants {", ".join(self.shot.stems)}')
        return time.time()

    def record_stop(self):
        for p in (self.rec, self.aud, *getattr(self, 'stem_recs', {}).values()):
            if p:
                try:
                    os.killpg(p.pid, 2)
                except ProcessLookupError:
                    pass
        for p in (self.rec, self.aud, *getattr(self, 'stem_recs', {}).values()):
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
        amap = ['-map', '1:a']
        stems = self.mux_stems(raw, duration)
        if stems:
            # the take's own track stays the whole of its sound: every stem, summed at unity
            audio_in = []
            for p in stems.values():
                audio_in += ['-i', p]
            amap = ['-filter_complex', ''.join(f'[{i + 1}:a]' for i in range(len(stems))) +
                    f'amix=inputs={len(stems)}:normalize=0:duration=longest[a]', '-map', '[a]']
        x264 = ['-preset', 'veryfast', '-crf', '18'] if self.tier == 'probe' else ['-preset', 'slow', '-crf', '14']
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-ss', f'{lead:.3f}', '-i', fixed, *audio_in,
                        '-map', '0:v', *amap, '-fps_mode', 'passthrough',
                        '-c:v', 'libx264', *x264, '-pix_fmt', 'yuv420p',
                        '-color_range', 'tv', '-colorspace', 'bt470bg', '-color_primaries', 'bt470bg',
                        '-color_trc', 'smpte170m', '-c:a', 'pcm_s16le', '-ar', '48000', '-ac', '2',
                        '-t', f'{duration:.3f}', self.stem + '.mkv'], check=True)

    def mux_stems(self, raw, duration):
        """Each stem's recording -> <stem>.<cat>.wav, cut from its end like the picture, so
        every stem and the take share one time line: {cat: path}, effects first. Nothing
        unless the session recorded stems. The session's own sink is the effects stem."""
        for cat in sb.CATEGORIES:       # an earlier take's stems are not this one's
            try:
                os.remove(f'{self.stem}.{cat}.wav')
            except FileNotFoundError:
                pass
        recs = getattr(self, 'stem_recs', {})
        if not recs:
            return {}
        out = {}
        for cat, src in [('effects', raw + '.wav')] + [(c, f'{raw}.{c}.wav') for c in sorted(recs)]:
            dst = f'{self.stem}.{cat}.wav'
            if os.path.exists(src) and os.path.getsize(src) > 1000:
                inp = ['-ss', f'{max(0.0, probe_duration(src) - duration):.3f}', '-i', src]
            else:
                inp = ['-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo']
                self.note(f'stem {cat}: nothing recorded, silence')
            subprocess.run(['ffmpeg', '-v', 'error', '-y', *inp, '-af', 'apad', '-c:a', 'pcm_s16le', '-ar', '48000',
                            '-ac', '2', '-t', f'{duration:.3f}', dst], check=True)
            out[cat] = dst
        return out

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
        try:
            for attempt in (1, 2):
                try:
                    return self.attempt()
                except GameDied as e:
                    self.cleanup_attempt()
                    if attempt == 2:
                        self.note(f'FAILED: {e} (second try)')
                        return False
                    self.note(f'{e}; trying once more')
                except Exception as e:      # one failed take must not end the others
                    self.note(f'FAILED: {e}')
                    return False
        finally:
            self.log.close()

    def cleanup_attempt(self):
        self.rec = None          # attempt()'s `finally` has stopped everything of the attempt
        self.sid = None
        self.sess = None

    def attempt(self):
        raw = tempfile.mkdtemp(prefix=f'{self.variant}-', dir=os.path.dirname(self.stem)) + '/raw'
        ok = False
        try:
            self.start()
            self.wait_ready()
            self.follow()
            self.run(self.shot.setup)
            self.check_alive()
            self.record_start(raw)
            self.wait_recording(raw)
            t0 = time.time()
            events = []
            self.run(self.shot.action, events, t0)
            time.sleep(TAIL)
            self.check_alive()
            t1 = time.time()
            duration = t1 - t0
            self.record_stop()
            self.gather()
            self.stop()
            self.sid = None
            self.mux(raw, duration)
            json.dump(dict(start=t0, duration=round(duration, 3), events=events),
                      open(self.stem + '.events.json', 'w'), indent=1)
            self.write_marks(t0, duration, events)
            json.dump(dict(hash=self.film.spec_hash(self.shot, self.variant, self.tier),
                           spec=self.film.spec(self.shot, self.variant, self.tier), **self.code,
                           res=self.film.res(self.tier, self.shot), tier=self.tier,
                           filmed=time.strftime('%Y-%m-%d %H:%M:%S')),
                      open(self.stem + '.spec.json', 'w'), indent=1)
            self.contact_sheet(probe_duration(self.stem + '.mkv'))
            self.note(f'done, {duration:.1f} s -> {self.stem}.mkv')
            ok = True
        finally:
            if getattr(self, 'rec', None):
                self.record_stop()
            if self.tail:
                self.tail.stop()
            self.stop()
            shutil.rmtree(os.path.dirname(raw), ignore_errors=True)
        return ok


class Ensemble:
    """A shot with Players: one Take per player, started together, set up and recorded
    over one action, so their time lines agree and the layout can put them side by side.
    A line `host: ...` goes to that player; any other line to each in turn, and a shared
    `wait` is waited once."""

    def __init__(self, film, shot, tier):
        self.film, self.shot, self.tier = film, shot, tier
        self.takes = {p: Take(film, shot, p, tier) for p in shot.players}
        for t in self.takes.values():
            t.ensemble = self
        self.tag = f'{shot.id}/{",".join(shot.players)}'

    def run(self, lines, events=None, t0=None):
        for ln, text in lines:
            who, line = self.shot.split_line(text)
            if events is not None:
                events.append(dict(t=round(time.time() - t0, 3), op=line.partition(' ')[0],
                                   arg=line.partition(' ')[2], player=who))
            if who:
                self.takes[who].run([(ln, line)])
            elif line.startswith('wait '):
                time.sleep(float(line.split()[1]))
            else:
                for t in self.takes.values():
                    t.run([(ln, line)])
            if events is not None:
                events[-1]['t1'] = round(time.time() - t0, 3)

    def __call__(self):
        takes = list(self.takes.values())
        for t in takes:
            os.makedirs(os.path.dirname(t.stem), exist_ok=True)
            t.log = open(t.stem + '.log', 'w')
        try:
            for attempt in (1, 2):
                try:
                    return self.attempt()
                except GameDied as e:
                    for t in takes:
                        t.rec, t.sid, t.sess = None, None, None
                    if attempt == 2:
                        for t in takes:
                            t.note(f'FAILED: {e} (second try)')
                        return False
                    for t in takes:
                        t.note(f'{e}; trying the whole shot once more')
        finally:
            for t in takes:
                t.log.close()

    def attempt(self):
        takes = list(self.takes.values())
        raws = {}
        ok = False
        for t in takes:
            raws[t] = tempfile.mkdtemp(prefix=f'{t.variant}-', dir=os.path.dirname(t.stem)) + '/raw'
        # every player's slot at once, or none: two shots each holding one and waiting for
        # another would wait for ever (slots.py)
        held = slots.acquire(self.tag, n=len(takes), say=lambda m: takes[0].note(m.splitlines()[0]))
        for t, slot in zip(takes, held):
            t.slot = slot
        try:
            # the players' sessions start together (a start is mostly waiting on the clone
            # and Wine), staggered: never two starts in one second
            def bring_up(i_t):
                i, t = i_t
                time.sleep(8 * i)
                t.start()
                t.wait_ready()
            with ThreadPoolExecutor(len(takes)) as ex:
                for f in [ex.submit(bring_up, it) for it in enumerate(takes)]:
                    f.result()
            for t in takes:
                t.follow()
            self.run(self.shot.setup)
            for t in takes:
                t.check_alive()
            for t in takes:
                t.record_start(raws[t])
            for t in takes:
                t.wait_recording(raws[t])
            t0 = time.time()
            events = []
            self.run(self.shot.action, events, t0)
            time.sleep(TAIL)
            for t in takes:
                t.check_alive()
            duration = time.time() - t0
            for t in takes:
                t.record_stop()
            for t in takes:
                t.gather()
                t.stop()
                t.sid = None
            for t in takes:
                t.mux(raws[t], duration)
                mine = [e for e in events if e.get('player') in (None, t.variant)]
                json.dump(dict(start=t0, duration=round(duration, 3), events=mine),
                          open(t.stem + '.events.json', 'w'), indent=1)
                t.write_marks(t0, duration, mine)
                json.dump(dict(hash=self.film.spec_hash(self.shot, t.variant, self.tier),
                               spec=self.film.spec(self.shot, t.variant, self.tier), **t.code,
                               res=self.film.res(self.tier, self.shot), tier=self.tier,
                               filmed=time.strftime('%Y-%m-%d %H:%M:%S')),
                          open(t.stem + '.spec.json', 'w'), indent=1)
                t.contact_sheet(probe_duration(t.stem + '.mkv'))
                t.note(f'done, {duration:.1f} s -> {t.stem}.mkv')
            ok = True
        except GameDied:
            raise
        except Exception as e:
            for t in takes:
                t.note(f'FAILED: {e}')
        finally:
            for t in takes:
                if t.slot:              # a start that never got to hand it down
                    t.slot.close()
                    t.slot = None
                if getattr(t, 'rec', None):
                    t.record_stop()
                if t.tail:
                    t.tail.stop()
                t.stop()
                shutil.rmtree(os.path.dirname(raws[t]), ignore_errors=True)
        return ok


def film_takes(a, tier):
    film = load(a.board)
    probs = [(s, m) for s, m in sb.check(film) if s is None or not a.shots or s.id in a.shots]
    if probs:
        for s, m in probs:
            print(f'! {s.id if s else "film"}: {m}')
        sys.exit('a2film: the board has problems (./a2film check)')
    todo = []
    run_tier = tier
    for s in select(film, a.shots):
        if not s.filmed:
            continue
        tier = film.shot_tier(s, run_tier)      # a probe run films a `Tier: final` shot's final take
        if s.players:       # all of a shot's players are filmed together, or none
            states = {p: take_check(film, s, p, tier, a.ignore_code) for p in s.players}
            if a.force or any(st != 'ok' for st, _ in states.values()):
                for p, (st, why) in states.items():
                    if why:
                        say(f'{s.id}/{p}', why)
                todo.append(Ensemble(film, s, tier))
            else:
                say(f'{s.id}', 'current, skipped (--force to re-film)')
            continue
        for v in s.variants:
            if a.variant and v not in a.variant:
                continue
            st, why = take_check(film, s, v, tier, a.ignore_code)
            if not a.force and st == 'ok':
                say(f'{s.id}/{v}', 'current, skipped (--force to re-film)')
                continue
            if why:
                say(f'{s.id}/{v}', why)
            todo.append(Take(film, s, v, tier))
    if not todo:
        print('nothing to film')
        return 0
    # takes larger than the run's own resolution (a shot's `Tier:` above the probes) run
    # after the others, one at a time: several large recording games at once is what died
    # at 1080p (films/README.md, "Traps")
    base = film.res(run_tier)

    def res_of(t):
        return film.res(t.tier, t.shot)

    def px(r):
        w, h = r.lower().split('x')
        return int(w) * int(h)

    light = [t for t in todo if px(res_of(t)) <= px(base)]
    heavy = [t for t in todo if px(res_of(t)) > px(base)]
    say('a2film', (f'{len(light)} takes at {base}, {a.jobs} at once' if light else 'no takes at ' + base)
                  + f'; at most {slots.count()[0]} games run on the machine at a time, the rest wait for a '
                    f'slot (./a2film slots)')
    if heavy:
        say('a2film', f'then {len(heavy)} at their own tier, one at a time: '
                      + ', '.join(f'{t.tag} {res_of(t)}' for t in heavy))

    def staggered(i_take):
        i, t = i_take
        time.sleep(min(i, a.jobs - 1) * 8 if i < a.jobs else 0)   # never two starts in one second
        return t()

    with ThreadPoolExecutor(a.jobs) as ex:
        results = list(ex.map(staggered, enumerate(light)))
    results += [t() for t in heavy]
    todo = light + heavy
    bad = [t.tag for t, ok in zip(todo, results) if not ok]
    sus = [x.tag for t, ok in zip(todo, results) if ok
           for x in (t.takes.values() if isinstance(t, Ensemble) else [t]) if x.suspect]
    say('a2film', f'{len(todo) - len(bad)} of {len(todo)} takes filmed' + (f'; failed: {", ".join(bad)}' if bad else '')
        + (f'; suspect (Expect not met): {", ".join(sus)}' if sus else ''))
    return 1 if bad or sus else 0


OPTS = [(('--jobs', '-j'), dict(type=int)),
        (('--force',), dict(action='store_true', help='re-film takes that are current')),
        (('--variant',), dict(action='append', help='only this variant (repeatable)')),
        IGNORE_CODE]


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
