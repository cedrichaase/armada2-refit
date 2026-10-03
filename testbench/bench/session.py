"""One bench session: a private copy of the game on a private headless display.

    clone     reflink copies of the game directory and the Wine prefix -- the user's
              install is never written, and a run can do anything to its copy
    prepare   the resolution into ARMADA.PRF; the resolution-dependent layers (HUD
              canvas, cursors, font) rebuilt for it; or a2mod stock
    display   sway, headless, one output at exactly the resolution under test, with
              Xwayland inside it -- the same Xwayland path the game takes under Hyprland
    input     a2input: a virtual pointer and keyboard on that sway's seat
    game      launched through umu, exactly as Heroic launches it
    teardown  game, display and input stopped; logs gathered; the clone deleted

A session outlives the process that started it (`a2test session start` returns while
the game keeps running), so everything needed to reattach is in session.json.
"""
import datetime
import fcntl
import glob
import itertools
import json
import os
import re
import shutil
import signal
import subprocess
import threading
import uuid
import time
from pathlib import Path

from . import config
from .log import Log


def now():
    return datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')


def pid_alive(pid):
    if not pid:
        return False
    try:
        os.kill(pid, 0)
    except OSError:
        return False
    # a zombie is not alive
    try:
        return Path(f'/proc/{pid}/stat').read_text().split(') ', 1)[1][0] != 'Z'
    except OSError:
        return False


def kill_group(pid, sig=signal.SIGTERM):
    """Everything here is started with setsid, so its pid is its process group -- but a
    pid out of session.json may be long gone and reused.  Only a group that pid still
    LEADS, and never the bench's own, is signalled."""
    if not pid or pid in (os.getpid(), os.getpgrp()):
        return
    try:
        pgid = os.getpgid(pid)
    except OSError:
        pgid = pid          # the leader has exited; its group may live on without it
        if not _group_has_members(pgid):
            return
    if pgid != pid or pgid == os.getpgrp():
        return
    try:
        os.killpg(pgid, sig)
    except OSError:
        pass


def _group_pids(pgid):
    out = set()
    if not pgid:
        return out
    for d in glob.glob('/proc/[0-9]*/stat'):
        try:
            if int(Path(d).read_text().rsplit(')', 1)[1].split()[2]) == pgid:
                out.add(int(d.split('/')[2]))
        except (OSError, ValueError, IndexError):
            continue
    return out


def _group_has_members(pgid):
    for d in glob.glob('/proc/[0-9]*/stat'):
        try:
            if int(Path(d).read_text().rsplit(')', 1)[1].split()[2]) == pgid:
                return True
        except (OSError, ValueError, IndexError):
            continue
    return False


class GameError(Exception):
    pass


def checkout_provenance(path):
    """A checkout given to --install: its absolute path and the commit it is at, with
    `-dirty` when it has uncommitted changes -- the report records exactly this."""
    p = Path(path).expanduser().resolve()
    if not os.access(p / 'install', os.X_OK):
        raise GameError(f'{p}: no executable ./install (not a checkout of either repository?)')
    r = subprocess.run(['git', '-C', str(p), 'describe', '--always', '--dirty', '--abbrev=10'],
                       capture_output=True, text=True)
    return dict(path=str(p), describe=r.stdout.strip() if r.returncode == 0 else 'not a git checkout')


class Session:
    def __init__(self, state, statefile):
        self.s = state
        self.statefile = Path(statefile)
        self.dir = Path(state['dir'])            # artifacts: kept
        self.work = Path(state['work'])          # clone, sockets, fifos: deleted
        self.res = tuple(state['res'])
        self.log = Log(self.dir)

    # ------------------------------------------------------------------ lifecycle

    @classmethod
    def create(cls, artifacts_dir, res, mod='refit', vnc=False, record=False,
               audio=False, keep=False, label='', stock_shell=None, installs=None, assets=None):
        # timestamp and pid alone collide when a run starts cases in parallel threads
        with _SID_LOCK:
            n = next(_SID_SEQ)
        sid = datetime.datetime.now().strftime('%Y%m%d-%H%M%S-') + f'{os.getpid() % 100000}' + (f'-{n}' if n else '')
        work = config.CACHE / 'sessions' / sid
        artifacts_dir = Path(artifacts_dir)
        for d in (work, artifacts_dir / 'shots', artifacts_dir / 'logs'):
            d.mkdir(parents=True, exist_ok=True)
        # assets='none': the game as someone without a texture pack gets it -- A2_DATA is
        # an empty directory, and this checkout's ./install runs into the clone unless
        # --install names others.
        if assets == 'none' and mod == 'refit' and not installs:
            installs = [config.REPO]
        installs = [checkout_provenance(p) for p in (installs or [])]
        state = dict(id=sid, dir=str(artifacts_dir), work=str(work), res=list(res), mod=mod, stock_shell=stock_shell,
                     installs=installs, assets=assets,
                     vnc=vnc, record=record, audio=audio, keep=keep, label=label,
                     created=now(), pids={}, env={}, shots=0, game_started=None,
                     exception_mtime=None, vnc_port=None)
        sess = cls(state, work / 'session.json')
        sess.save()
        (artifacts_dir / 'session.json').write_text(json.dumps({'statefile': str(sess.statefile)}))
        sess.log.meta(session=sid, resolution=config.res_name(res),
                      aspect=config.aspect_name(res), mod=mod, label=label, started=now(),
                      **({'assets': assets} if assets else {}),
                      **({'installs': [f"{i['path']} @ {i['describe']}" for i in installs]} if installs else {}))
        return sess

    @classmethod
    def load(cls, statefile):
        state = json.loads(Path(statefile).read_text())
        return cls(state, statefile)

    def save(self):
        tmp = self.statefile.with_suffix('.tmp')
        tmp.write_text(json.dumps(self.s, indent=1))
        tmp.replace(self.statefile)

    @property
    def game_dir(self):
        return self.work / 'game'

    @property
    def prefix_dir(self):
        return self.work / 'prefix'

    # ------------------------------------------------------------------ clone + prepare

    def clone(self):
        t = time.time()
        for src, dst in ((config.GAME, self.game_dir), (config.PREFIX, self.prefix_dir)):
            if not src.is_dir():
                raise GameError(f'no {src}')
            r = subprocess.run(['cp', '-a', '--reflink=always', str(src), str(dst)],
                               capture_output=True, text=True)
            if r.returncode:
                raise GameError(f'reflink clone of {src} failed (must be btrfs, same '
                                f'filesystem as {config.CACHE}): {r.stderr.strip()}')
        # vkBasalt's config lives outside the game directory, and a2mod toggles it; the
        # session gets its own copy so neither touches the user's.
        if config.VKBASALT.is_dir():
            shutil.copytree(config.VKBASALT, self.work / 'xdg' / 'a2-vkbasalt', symlinks=True)
        exc = self.game_dir / 'exception.txt'
        self.s['exception_mtime'] = exc.stat().st_mtime if exc.exists() else None
        self.save()
        self.log.action(f'cloned the game and prefix (reflink) in {time.time() - t:.1f}s',
                        detail=f'{config.GAME} -> {self.game_dir}\n{config.PREFIX} -> {self.prefix_dir}')

    def prepare(self):
        w, h = self.res
        set_prf_resolution(self.game_dir / 'ARMADA.PRF', w, h)
        self.log.action(f'ARMADA.PRF resolution set to {w}x{h}')
        data = config.DATA
        if self.s.get('assets') == 'none':
            data = self.work / 'no-assets'
            data.mkdir(exist_ok=True)
        env = dict(os.environ, A2_GAME=str(self.game_dir), A2_GAME_DIR=str(self.game_dir),
                   A2_DATA=str(data),
                   XDG_DATA_HOME=str(self.work / 'xdg'), TMPDIR=str(self.work))
        plog = self.dir / 'logs' / 'prepare.log'
        installs = self.s.get('installs') or []
        if self.s['mod'] == 'stock':
            steps = [[str(config.REPO / 'a2mod'), 'stock']]
        elif installs:
            # Checkouts stacked with --install: a known baseline first -- stock, whatever
            # the user's install happens to carry -- then each checkout's ./install, in
            # the order given, into the clone. So what the case shows is those commits
            # and nothing else, and the report names them.
            steps = [[str(config.REPO / 'a2mod'), 'stock']] + \
                    [[str(Path(i['path']) / 'install')] for i in installs]
        else:
            # HUD.asi fixes the HUD, font and cursors at run time for whatever mode
            # is set, so the install takes no resolution: the same install is what
            # every case runs, which is the point.  install.sh first reverts the
            # three file-based fixes it replaced, which the user's install may carry.
            steps = [[str(config.REPO / 'hud/install.sh')]]
        with open(plog, 'a') as f:
            for cmd in steps:
                f.write(f'$ {" ".join(cmd)}\n')
                f.flush()
                r = subprocess.run(cmd, env=env, stdout=f, stderr=subprocess.STDOUT,
                                   cwd=str(Path(cmd[0]).parent))
                f.write(f'[exit {r.returncode}]\n\n')
                name = Path(cmd[0]).name
                if name == 'install':
                    i = next(i for i in installs if Path(i['path']) / 'install' == Path(cmd[0]))
                    name = f"{i['path']}/install @ {i['describe']}"
                if r.returncode:
                    self.log.action(f'prepare: {name} failed (exit {r.returncode})',
                                    status='fail', detail=f'see logs/prepare.log')
                    raise GameError(f'{name} failed; see {plog}')
                self.log.action(f'prepare: {name} {" ".join(cmd[1:])}', detail='logs/prepare.log')
        if self.s['mod'] == 'stock' and self.s.get('stock_shell') == 'embed':
            self.embed_stock_shell()

    def embed_stock_shell(self):
        """Stock, except that the menus are one window with the game (menus/README.md,
        "One window"). Without Embed each menu is its own X11 window and injected clicks
        mostly never reach it: the stock Borg campaign click did nothing, twice
        (2026-09-26). Menus.asi goes back with nothing else on: no scaling, the stock
        800x600 shell mode, no backdrops, stock Esc. The HUD, font and cursors, which the
        stock baseline exists for, stay stock."""
        shutil.copy2(config.GAME / 'Menus.asi', self.game_dir / 'Menus.asi')
        (self.game_dir / 'Menus.ini').write_text(
            '; written by the Armada II test bench: stock shell, embedded only\r\n'
            '[Menus]\r\nMode=1\r\nDesignWidth=800\r\nDesignHeight=600\r\nIntegerScale=1\r\n'
            'Smooth=0\r\nRaiseShellMode=0\r\nEmbed=1\r\nEscapeReturns=0\r\nUnderlay=0\r\n'
            'Log=1\r\nTrace=0\r\nBackdrops=0\r\n')
        self.log.action('stock shell embedded: Menus.asi with Embed=1 and nothing else, so menus take clicks')

    # ------------------------------------------------------------------ display

    def start_display(self):
        w, h = self.res
        rt = self.work / 'rt'
        rt.mkdir(mode=0o700, exist_ok=True)
        envfile = self.work / 'sway.env'
        conf = self.work / 'sway.conf'
        conf.write_text(f"""\
# generated by the Armada II test bench
output HEADLESS-1 mode --custom {w}x{h}@60Hz
output HEADLESS-1 bg #000000 solid_color
default_border none
default_floating_border none
focus_follows_mouse no
xwayland force
seat seat0 hide_cursor 0
exec sh -c 'env > {envfile}.tmp && mv {envfile}.tmp {envfile}'
""")
        env = dict(HOME=str(config.HOME), PATH=os.environ['PATH'], XDG_RUNTIME_DIR=str(rt),
                   WLR_BACKENDS='headless', WLR_HEADLESS_OUTPUTS='1',
                   WLR_LIBINPUT_NO_DEVICES='1', WLR_RENDER_DRM_DEVICE=_render_node())
        slog = open(self.dir / 'logs' / 'sway.log', 'w')
        p = subprocess.Popen(['sway', '-c', str(conf)], env=env, stdout=slog, stderr=subprocess.STDOUT,
                             stdin=subprocess.DEVNULL, start_new_session=True, cwd=str(self.work))
        self.s['pids']['sway'] = p.pid
        for _ in range(100):
            if envfile.exists():
                break
            if p.poll() is not None:
                raise GameError('sway exited during start-up; see logs/sway.log')
            time.sleep(0.1)
        else:
            raise GameError('sway did not come up in 10s; see logs/sway.log')
        senv = dict(l.split('=', 1) for l in envfile.read_text().splitlines() if '=' in l)
        self.s['env'] = dict(DISPLAY=senv['DISPLAY'], SWAYSOCK=senv['SWAYSOCK'],
                             WAYLAND_DISPLAY=str(rt / senv['WAYLAND_DISPLAY']),
                             XDG_RUNTIME_DIR=str(rt))
        self.save()
        self.log.action(f'headless display up: sway output HEADLESS-1 at {w}x{h}, '
                        f'Xwayland {self.s["env"]["DISPLAY"]}')
        self.start_input()
        if self.s['vnc']:
            self.start_vnc()
        if self.s['record']:
            self.start_recording()

    def wl_env(self):
        e = self.s['env']
        return dict(os.environ, WAYLAND_DISPLAY=e['WAYLAND_DISPLAY'],
                    XDG_RUNTIME_DIR=e['XDG_RUNTIME_DIR'], SWAYSOCK=e['SWAYSOCK'])

    def swaymsg(self, *args):
        return subprocess.run(['swaymsg', '-s', self.s['env']['SWAYSOCK'], *args],
                              capture_output=True, text=True).stdout

    def start_input(self):
        build_input()
        infifo, outfifo = self.work / 'input.in', self.work / 'input.out'
        for f in (infifo, outfifo):
            if not f.exists():
                os.mkfifo(f)
        # O_RDWR on both ends: the daemon never sees EOF between clients, and a client's
        # open never blocks waiting for the other side.
        fin = os.open(infifo, os.O_RDWR)
        fout = os.open(outfifo, os.O_RDWR)
        err = open(self.dir / 'logs' / 'input.log', 'w')
        p = subprocess.Popen([str(config.INPUT_BIN), str(self.res[0]), str(self.res[1])],
                             stdin=fin, stdout=fout, stderr=err, env=self.wl_env(),
                             start_new_session=True)
        os.close(fin)
        os.close(fout)
        self.s['pids']['input'] = p.pid
        self.save()
        if self._input_reply(timeout=5) != 'ready':
            raise GameError('a2input did not start; see logs/input.log')

    def start_vnc(self):
        if not shutil.which('wayvnc'):
            self.log.note('VNC requested but wayvnc is not installed (pacman -S wayvnc); continuing without it',
                          status='warn')
            return
        with _VNC_LOCK:            # parallel cases: a port one of them has claimed may not be bound yet
            port = config.VNC_BASE_PORT
            while port in _VNC_CLAIMED or _port_busy(port):
                port += 1
            _VNC_CLAIMED.add(port)
        vlog = open(self.dir / 'logs' / 'wayvnc.log', 'w')
        p = subprocess.Popen(['wayvnc', '--disable-input', '-o', 'HEADLESS-1', '127.0.0.1', str(port)],
                             env=self.wl_env(), stdout=vlog, stderr=subprocess.STDOUT,
                             start_new_session=True)
        self.s['pids']['vnc'] = p.pid
        self.s['vnc_port'] = port
        self.save()
        self.log.note(f'VNC (view-only) on 127.0.0.1:{port} -- e.g. `vncviewer localhost:{port}`')
        print(f'  watch: vncviewer localhost:{port}', flush=True)

    def start_recording(self):
        if not shutil.which('wf-recorder'):
            self.log.note('recording requested but wf-recorder is not installed (pacman -S wf-recorder)',
                          status='warn')
            return
        out = self.dir / 'run.mp4'
        rlog = open(self.dir / 'logs' / 'wf-recorder.log', 'w')
        p = subprocess.Popen(['wf-recorder', '-o', 'HEADLESS-1', '-y', '-r', '15', '-f', str(out)],
                             env=self.wl_env(), stdout=rlog, stderr=subprocess.STDOUT,
                             stdin=subprocess.DEVNULL, start_new_session=True)
        self.s['pids']['record'] = p.pid
        self.save()
        self.log.note('recording the run to run.mp4')

    # ------------------------------------------------------------------ input

    def _input_reply(self, timeout=10):
        fd = os.open(self.work / 'input.out', os.O_RDONLY | os.O_NONBLOCK)
        buf = b''
        end = time.time() + timeout
        try:
            while time.time() < end:
                try:
                    chunk = os.read(fd, 4096)
                except BlockingIOError:
                    chunk = b''
                buf += chunk
                if b'\n' in buf:
                    return buf.split(b'\n', 1)[0].decode()
                time.sleep(0.01)
        finally:
            os.close(fd)
        return None

    def input(self, cmd):
        """Send one command to a2input and wait for its answer."""
        if not pid_alive(self.s['pids'].get('input')):
            raise GameError('the input daemon is not running')
        with open(self.work / 'input.lock', 'w') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            # drop any answer a previous, interrupted client never collected
            fd = os.open(self.work / 'input.out', os.O_RDONLY | os.O_NONBLOCK)
            try:
                while os.read(fd, 4096):
                    pass
            except BlockingIOError:
                pass
            finally:
                os.close(fd)
            with open(self.work / 'input.in', 'w') as f:
                f.write(cmd.rstrip('\n') + '\n')
            r = self._input_reply(timeout=30)
        if r != 'ok':
            raise GameError(f'input "{cmd}": {r or "no answer"}')

    def move(self, x, y):
        """Put the pointer at x,y -- by relative motion once its position is known.
        In a mission the game keeps its own cursor and moves it by (absolute position -
        a reference point it fixes once, then tries to warp back to); Xwayland cannot
        warp a virtual pointer, so absolute moves arrived as ever-growing jumps and the
        cursor piled into the corner (+50 across drew +200, +250, +300 ...).  Relative
        motion moved it exactly, and moves the compositor's pointer the same way, so
        the shell's Win32 menus see the same position (measured 2026-09-26)."""
        w, h = self.res
        x = min(max(int(round(x)), 0), w - 1)
        y = min(max(int(round(y)), 0), h - 1)
        cur = self.s.get('cursor')
        if cur is None:
            self.input(f'move {x} {y}')
        elif (x, y) != tuple(cur):
            self.input(f'rel {x - cur[0]} {y - cur[1]}')
        self.s['cursor'] = [x, y]
        self.save()

    def glide(self, x, y, steps=8):
        """Move there in a few steps: the menu buttons only light up (and the shell only
        tracks the hover) on WM_MOUSEMOVE inside them, which a single jump can skip."""
        cx, cy = self.s.get('cursor') or [x - 40, y]
        for i in range(1, steps + 1):
            self.move(cx + (x - cx) * i / steps, cy + (y - cy) * i / steps)
            time.sleep(0.02)

    def click(self, x, y, button=1, double=False):
        self.glide(x, y)
        time.sleep(0.15)
        for _ in range(2 if double else 1):
            self.input(f'down {button}')
            time.sleep(0.08)
            self.input(f'up {button}')
            time.sleep(0.08)

    def drag(self, x0, y0, x1, y1, button=1):
        """Press at one point, glide to the other with the button held, release: a
        selection box in a mission."""
        self.glide(x0, y0)
        time.sleep(0.15)
        self.input(f'down {button}')
        time.sleep(0.1)
        self.glide(x1, y1, steps=16)
        time.sleep(0.15)
        self.input(f'up {button}')
        time.sleep(0.1)

    def key(self, combo):
        self.input(f'key {combo}')

    def type(self, text):
        self.input(f'type {text}')

    def wheel(self, n):
        self.input(f'wheel {n}')

    # ------------------------------------------------------------------ game

    def launch(self, args='-nointro'):
        if self.game_pid():
            raise GameError('the game is already running in this session')
        g = self.game_dir
        for name in ('Menus.log', 'MSAA.log', 'HUD.log', 'BinkProxy.log', 'Armada2_d3d9.log'):
            try:
                (g / name).unlink()
            except FileNotFoundError:
                pass
        logs = self.dir / 'logs'
        e = self.s['env']
        env = {k: v for k, v in os.environ.items()
               if k not in ('WAYLAND_DISPLAY', 'SWAYSOCK', 'WINEPREFIX', 'LD_PRELOAD')}
        env.update(
            DISPLAY=e['DISPLAY'],
            GAMEID='umu-0', STORE='gog', PROTONPATH=str(config.PROTON),
            WINEPREFIX=str(self.prefix_dir), STEAM_COMPAT_DATA_PATH=str(self.prefix_dir),
            STEAM_COMPAT_INSTALL_PATH=str(g),
            STEAM_COMPAT_CLIENT_INSTALL_PATH=str(config.HOME / '.local/share/.steam/steam'),
            STEAM_COMPAT_APP_ID='0', SteamAppId='0', SteamGameId='a2test',
            UMU_RUNTIME_UPDATE='0',
            WINEDLLOVERRIDES=config.DLL_OVERRIDES,
            WINEDEBUG=os.environ.get('A2TEST_WINEDEBUG', '-all,err+all'),
            WINE_FULLSCREEN_FSR='0',
            DXVK_LOG_PATH=str(logs), DXVK_LOG_LEVEL='info',
        )
        vkconf = self.work / 'xdg' / 'a2-vkbasalt' / 'vkBasalt.conf'
        if vkconf.exists():
            env.update(ENABLE_VKBASALT='1', VKBASALT_CONFIG_FILE=str(vkconf),
                       VKBASALT_LOG_FILE=str(logs / 'vkBasalt.log'), VKBASALT_LOG_LEVEL='info')
        if not self.s['audio']:
            # Silence by giving the game a device that plays nowhere.  History, all measured:
            # - PULSE_SINK at a null sink: winepulse ignores it and uses the server default;
            #   the user heard the menu music and the Borg cutscene.
            # - Disabling winepulse and winealsa: Proton-CachyOS's winepipewire.drv, which
            #   this prefix uses, still played the menu music (2026-09-26).
            # - Disabling every driver: Armada2.exe dies at start-up without an audio
            #   device (`virtual_setup_exception stack overflow`, every case).
            # So: a null sink of this session's on the real server, pre-registered in the
            # clone's registry under our own GUID and made Wine's default output
            # (mmdevapi reads Software\Wine\Drivers\<drv>\DefaultOutput).  The watchdog
            # checks the game's stream lands there and moves it there if not.
            sink = self.start_null_sink()
            env['WINEDLLOVERRIDES'] = config.DLL_OVERRIDES + ';winealsa.drv=d;wineoss.drv=d'
            route_wine_audio(self.prefix_dir / 'pfx' / 'user.reg', sink)
        # a marker the watchdog can match whatever pid namespace the game ends up in
        env['PULSE_PROP'] = f'a2test.session={self.s["id"]}'
        env['PIPEWIRE_PROPS'] = f'{{ a2test.session = "{self.s["id"]}" }}'
        (logs / 'launch-env.txt').write_text(
            f'cwd: {g}\nargv: python3 {config.UMU} {g / "Armada2.exe"} {args}\n\n' +
            ''.join(f'{k}={v}\n' for k, v in sorted(env.items())))
        wlog = open(logs / 'wine.log', 'a')
        wlog.write(f'--- {now()} launch Armada2.exe {args}\n')
        wlog.flush()
        p = subprocess.Popen(['python3', str(config.UMU), str(g / 'Armada2.exe'), *args.split()],
                             env=env, cwd=str(g), stdout=wlog, stderr=subprocess.STDOUT,
                             stdin=subprocess.DEVNULL, start_new_session=True)
        self.s['pids']['launcher'] = p.pid
        self.s['game_started'] = time.time()
        self.s['launch_args'] = args
        self.save()
        for _ in range(1200):
            if self.game_pid():
                break
            if p.poll() is not None:
                raise GameError(f'the launcher exited ({p.returncode}) before the game started; see logs/wine.log')
            time.sleep(0.1)
        else:
            raise GameError('Armada2.exe did not appear within 120s; see logs/wine.log')
        self.log.action(f'launched Armada2.exe {args} through umu (pid {self.game_pid()})' +
                        ('' if self.s['audio'] else f', audio to the null sink {self.s["null_sink"]["name"]}'))
        if not self.s['audio']:
            threading.Thread(target=self._audio_watchdog, daemon=True).start()
            self.assert_silent()

    # ------------------------------------------------------------------ audio

    def start_null_sink(self):
        """A sink on the real server that plays nowhere, one per session (parallel cases
        each get their own). Unloaded in teardown."""
        name = f'a2test-{self.s["id"]}'
        r = subprocess.run(['pactl', 'load-module', 'module-null-sink', f'sink_name={name}',
                            f'sink_properties=device.description={name}'],
                           capture_output=True, text=True)
        if r.returncode:
            raise GameError(f'could not create the silent sink: {r.stderr.strip()}')
        self.s['null_sink'] = dict(name=name, module=int(r.stdout.strip()))
        self.save()
        return name

    def stop_null_sink(self):
        ns = self.s.get('null_sink')
        if ns:
            subprocess.run(['pactl', 'unload-module', str(ns['module'])], capture_output=True)

    def _our_pids(self):
        """This session's processes, by host pid and by their pid inside umu's container
        (the pid a PipeWire client reports is the one it sees)."""
        pids = set(self.game_pids())
        pids.update(_group_pids(self.s['pids'].get('launcher')))
        for pid in list(pids):
            try:
                for line in Path(f'/proc/{pid}/status').read_text().splitlines():
                    if line.startswith('NSpid:'):
                        pids.update(int(x) for x in line.split()[1:])
            except OSError:
                pass
        return pids

    def audio_streams(self):
        """This session's playback streams: [(index, sink name, label)]."""
        pids = self._our_pids()
        sinks = _pactl_json('sinks')
        names = {it.get('index'): it.get('name') for it in sinks}
        out = []
        for it in _pactl_json('sink-inputs'):
            props = it.get('properties', {})
            try:
                pid = int(props.get('application.process.id', -1))
            except ValueError:
                pid = -1
            sink = names.get(it.get('sink'), '')
            ns = self.s.get('null_sink') or {}
            if pid in pids or props.get('a2test.session') == self.s['id'] or sink == ns.get('name'):
                out.append((it.get('index'), sink, f'#{it.get("index")} {props.get("application.name", "")} '
                                                    f'(pid {pid}) on {sink}'))
        return out

    def _silence(self):
        """Move any stream of ours that is not on our null sink onto it, and mute it.
        Returns the ones that had to be moved: each is a leak."""
        ns = (self.s.get('null_sink') or {}).get('name')
        leaks = []
        for idx, sink, label in self.audio_streams():
            if not sink:
                continue      # not linked to any sink yet, so inaudible; its 'change' event comes back here
            if sink != ns and not sink.startswith('a2test-'):   # any session's null sink is silent
                subprocess.run(['pactl', 'set-sink-input-mute', str(idx), '1'], capture_output=True)
                if ns:
                    subprocess.run(['pactl', 'move-sink-input', str(idx), ns], capture_output=True)
                leaks.append(label)
            elif not self.s.get('audio_routed'):
                self.s['audio_routed'] = label
                self.log.action(f"the game's audio goes to the silent sink: {label}")
        return leaks

    def _audio_watchdog(self):
        """For the whole case. `pactl subscribe` reports each new stream as it is created,
        so a stray one is moved and muted within milliseconds; the game is then stopped
        and the next step fails on it."""
        sub = subprocess.Popen(['pactl', 'subscribe'], stdout=subprocess.PIPE, text=True,
                               stderr=subprocess.DEVNULL)
        self._audio_sub = sub            # teardown kills it: readline() may wait on the next event
        try:
            leaks = self._silence()
            while not leaks and self.running():
                line = sub.stdout.readline()
                if not line:
                    break
                if 'sink-input' in line and ("'new'" in line or "'change'" in line):
                    leaks = self._silence()
            if leaks:
                self.log.action(f'the game played to a real output ({", ".join(leaks)}): moved to the '
                                'silent sink, muted, and stopped the game', status='fail')
                self.stop_game()
        finally:
            sub.kill()

    def assert_silent(self, seconds=6):
        """The first seconds, synchronously, so a leak fails the launch itself."""
        end = time.time() + seconds
        while time.time() < end:
            leaks = self._silence()
            if leaks:
                self.stop_game()
                raise GameError(f'the game played to a real output ({", ".join(leaks)}); moved, muted '
                                'and stopped it')
            time.sleep(0.1)

    def game_pids(self):
        """The game process of THIS session: argv[0] is the Windows path of its
        Armada2.exe (`X:\\...\\Armada2.exe`), and its environment names this session's
        prefix.

        Matching "Armada2.exe anywhere in the command line" is wrong in a way that looks
        like a crash: umu_run.py, the proton script and umu.exe all carry it as an
        argument and are gone within seconds, while the real game process appears ~4 s
        after launch.  Latching onto one of those made the game "stop running" 2-8 s in,
        every time, while it went on running unwatched (and outlived its teardown).
        Wine names the process `Armada2.exe` under umu but `Main` elsewhere
        (menus/stop-game.sh), so the name is not used either."""
        out = []
        pfx = str(self.prefix_dir).encode()
        for d in glob.glob('/proc/[0-9]*'):
            try:
                argv0 = Path(d, 'cmdline').read_bytes().split(b'\0', 1)[0]
                if not (argv0.endswith(b'\\Armada2.exe') or argv0.endswith(b'/Armada2.exe')):
                    continue
                if pfx in Path(d, 'environ').read_bytes():
                    out.append(int(d.rsplit('/', 1)[1]))
            except OSError:
                continue
        return [p for p in out if pid_alive(p)]

    def game_pid(self):
        p = self.game_pids()
        return p[0] if p else None

    def running(self):
        return self.game_pid() is not None

    def crashed(self):
        """exception.txt is where the game's crash handler writes; it ships non-empty,
        so a crash is a change of mtime."""
        exc = self.game_dir / 'exception.txt'
        if not exc.exists():
            return False
        return exc.stat().st_mtime != self.s.get('exception_mtime')

    def isolation_check(self):
        """Prove the game reads its own copy: no open file or mapping of the real install."""
        real = str(config.GAME)
        pid = self.game_pid()
        if not pid:
            return None
        hits = set()
        try:
            for line in Path(f'/proc/{pid}/maps').read_text().splitlines():
                if real in line:
                    hits.add(line.split(None, 5)[-1])
            for fd in Path(f'/proc/{pid}/fd').iterdir():
                try:
                    t = os.readlink(fd)
                    if t.startswith(real):
                        hits.add(t)
                except OSError:
                    pass
        except OSError:
            return None
        return sorted(hits)

    def quit_game(self, timeout=20):
        """Ask nicely (WM_DELETE_WINDOW -> WM_CLOSE), then stop the prefix.  Returns
        True if the game closed on its own."""
        if not self.running():
            return True
        self.swaymsg('[class="steam_proton"]', 'kill')
        for _ in range(timeout * 4):
            if not self.running():
                return True
            time.sleep(0.25)
        self.stop_game()
        return False

    def stop_game(self):
        env = dict(os.environ, WINEPREFIX=str(self.prefix_dir / 'pfx'))
        ws = config.PROTON / 'files/bin/wineserver'
        if ws.exists():
            subprocess.run([str(ws), '-k'], env=env, capture_output=True, timeout=30)
        kill_group(self.s['pids'].get('launcher'))
        for _ in range(40):
            if not self.game_pids():
                break
            time.sleep(0.25)
        for p in self.game_pids():
            try:
                os.kill(p, signal.SIGKILL)
            except OSError:
                pass
        kill_group(self.s['pids'].get('launcher'), signal.SIGKILL)

    # ------------------------------------------------------------------ capture

    def screenshot(self, name='shot'):
        self.s['shots'] += 1
        self.save()
        safe = re.sub(r'[^A-Za-z0-9_.-]+', '-', name).strip('-') or 'shot'
        path = self.dir / 'shots' / f'{self.s["shots"]:03d}-{safe}.png'
        r = subprocess.run(['grim', '-o', 'HEADLESS-1', str(path)], env=self.wl_env(),
                           capture_output=True, text=True)
        if r.returncode:
            raise GameError(f'grim failed: {r.stderr.strip()}')
        return path

    def last_shot(self):
        shots = sorted((self.dir / 'shots').glob('*.png'))
        return shots[-1] if shots else None

    def game_log(self, name):
        """A log the instrumented code writes beside Armada2.exe, or one of ours."""
        for p in (self.game_dir / name, self.dir / 'logs' / name):
            if p.exists():
                return p.read_text(errors='replace')
        return None

    def collect_logs(self):
        dst = self.dir / 'logs'
        g = self.game_dir
        for name in ('Menus.log', 'MSAA.log', 'HUD.log', 'BinkProxy.log', 'ARMADA.PRF', 'Menus.ini',
                     'MSAA.ini', 'HUD.ini', 'dxvk.conf', 'Online.log', 'Online.ini', 'OnlineServer.log'):
            if (g / name).exists():
                shutil.copy2(g / name, dst / name)
        if self.crashed():
            shutil.copy2(g / 'exception.txt', dst / 'exception.txt')
        started = self.s.get('game_started') or 0
        if (g / 'Logs').is_dir():
            for f in (g / 'Logs').iterdir():
                if f.is_file() and f.stat().st_mtime >= started:
                    (dst / 'game-Logs').mkdir(exist_ok=True)
                    shutil.copy2(f, dst / 'game-Logs' / f.name)

    def teardown(self):
        try:
            # unconditionally: whatever the liveness check believes, nothing of this
            # prefix may outlive the session (it would keep rewriting a deleted clone)
            if self.s['pids'].get('launcher') or self.running():
                self.stop_game()
        finally:
            sub = getattr(self, '_audio_sub', None)
            if sub:
                sub.kill()
            self.stop_null_sink()
            try:
                self.collect_logs()
            except Exception as e:  # never let log collection keep a display alive
                self.log.note(f'collecting logs failed: {e}', status='warn')
            for k in ('record', 'vnc', 'input', 'sway'):
                pid = self.s['pids'].get(k)
                if pid_alive(pid):
                    kill_group(pid, signal.SIGINT if k == 'record' else signal.SIGTERM)
                    for _ in range(40):
                        if not pid_alive(pid):
                            break
                        time.sleep(0.1)
                    kill_group(pid, signal.SIGKILL)
            self.s['ended'] = now()
            self.save()
            shutil.copy2(self.statefile, self.dir / 'logs' / 'session-state.json')
            if not self.s['keep']:
                for _ in range(5):
                    shutil.rmtree(self.work, ignore_errors=True)
                    if not self.work.exists():
                        break
                    time.sleep(1)


# ---------------------------------------------------------------------- helpers

def set_prf_resolution(prf, w, h):
    """ARMADA.PRF line 5 holds `... <w> <h> <bpp> ...` and the file carries an embedded
    NUL, so this works on bytes (platform/README.md, "Setting the resolution").  Fields 7
    and 8 of that line, the same walk hud/ui-widescreen.py's reader does."""
    data = Path(prf).read_bytes()
    lines = data.split(b'\r\n')
    f = lines[4].split(b' ')
    if len(f) < 9 or not (f[6].isdigit() and f[7].isdigit()):
        raise GameError(f'unexpected ARMADA.PRF line 5: {lines[4]!r}')
    f[6], f[7] = str(w).encode(), str(h).encode()
    lines[4] = b' '.join(f)
    Path(prf).write_bytes(b'\r\n'.join(lines))


def _pactl_json(what):
    r = subprocess.run(['pactl', '--format=json', 'list', what], capture_output=True, text=True)
    try:
        return json.loads(r.stdout or '[]')
    except json.JSONDecodeError:
        return []


def route_wine_audio(user_reg, sink):
    """Make `sink` Wine's default output in this prefix. mmdevapi names an endpoint
    `{0.0.0.00000000}.{GUID}` and keeps each device's GUID under
    Software\\Wine\\Drivers\\<drv>\\devices\\0,<sink>; pre-registering one of ours lets
    DefaultOutput name it before Wine has ever seen the sink."""
    u = uuid.uuid4()
    endpoint = '{0.0.0.00000000}.{' + str(u).upper() + '}'
    guid = 'hex:' + ','.join(f'{b:02x}' for b in u.bytes_le)
    for drv in ('winepipewire.drv', 'winepulse.drv'):
        base = r'Software\\Wine\\Drivers\\' + drv
        set_reg_value(user_reg, base + r'\\devices\\0,' + sink, 'guid', guid, raw=True)
        for name in ('DefaultOutput', 'DefaultVoiceOutput'):
            set_reg_value(user_reg, base, name, endpoint)


def set_reg_value(reg, key, name, value, raw=False):
    """Set a value in a Wine .reg file (wineserver not running): `key` as the file
    spells it, with doubled backslashes. A string unless `raw` (e.g. 'hex:01,02')."""
    reg = Path(reg)
    text = reg.read_text(encoding='utf-8', errors='surrogateescape')
    line = f'"{name}"=' + (value if raw else f'"{value}"')
    m = re.search(r'^\[' + re.escape(key) + r'\][^\n]*\n', text, re.M | re.I)
    if not m:
        text = text.rstrip('\n') + f'\n\n[{key}]\n{line}\n'
    else:
        end = text.find('\n[', m.end())
        end = len(text) if end < 0 else end + 1
        body = text[m.end():end]
        rx = re.compile(r'^"' + re.escape(name) + r'"=.*$', re.M | re.I)
        body = rx.sub(lambda _: line, body, 1) if rx.search(body) else line + '\n' + body
        text = text[:m.end()] + body + text[end:]
    reg.write_text(text, encoding='utf-8', errors='surrogateescape')


_SID_SEQ, _SID_LOCK = itertools.count(), threading.Lock()
_VNC_CLAIMED, _VNC_LOCK = set(), threading.Lock()


def build_input():
    src = [config.INPUT_SRC / n for n in ('a2input.c', 'build.sh', 'wlr-virtual-pointer-unstable-v1.xml',
                                          'virtual-keyboard-unstable-v1.xml')]
    if config.INPUT_BIN.exists() and all(s.stat().st_mtime <= config.INPUT_BIN.stat().st_mtime for s in src):
        return
    r = subprocess.run(['bash', str(config.INPUT_SRC / 'build.sh')], capture_output=True, text=True)
    if r.returncode:
        raise GameError(f'building a2input failed:\n{r.stdout}{r.stderr}')


def _render_node():
    nodes = sorted(glob.glob('/dev/dri/renderD*'))
    return os.environ.get('A2TEST_RENDER_NODE', nodes[0] if nodes else '')


def _port_busy(port):
    import socket
    with socket.socket() as s:
        return s.connect_ex(('127.0.0.1', port)) == 0


def _comm(pid):
    try:
        return Path(f'/proc/{pid}/comm').read_text().strip()
    except OSError:
        return ''
