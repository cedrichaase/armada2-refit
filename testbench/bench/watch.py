"""a2test watch -- the bench dashboard: every job on the bench, its games, the queue and
what finished, served on 127.0.0.1:5950 (testbench/README.md, "Watching").

    /                 the page (bench/dash.html), which needs nothing from the network
    /events           Server-Sent Events: the state (bench/dash.py) every two seconds
    /api/state        the same, once (JSON)
    /snap/<sid>.jpg   a session's screen, at most SNAP_TTL seconds old, kept in memory
    /ws/<sid>         a session's screen live: a WebSocket bridged to a wayvnc started on
                      demand and stopped once nobody watches; refused while a perf A/B runs
    /novnc/...        noVNC 1.6.0, vendored in testbench/vendor/
    /results/...      the runs' reports, heat maps and dump previews (A2TEST_RESULTS)
    /baselines/...    `a2test baseline compare` output (~/.cache/a2test/baselines)

Nothing a session shows is written anywhere by this: snapshots live in memory. Ctrl-C
stops the wayvncs it started and nothing else.

`--install-desktop` writes a desktop entry that runs the MAIN checkout's `a2test watch
--desktop`, whichever worktree it is installed from (README.md, "Watching").

The WebSocket bridge is here rather than `wayvnc --websocket` because that segfaults on
the first client (wayvnc 0.10.1 / neatvnc 1.0.1, a call through a null pointer -- measured).
"""
import argparse
import base64
import hashlib
import json
import mimetypes
import os
import resource
import shutil
import socket
import struct
import subprocess
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import unquote

from . import config, dash
from .session import GameError, _port_busy

PAGE = Path(__file__).with_name('dash.html')
NOVNC = config.BENCH / 'vendor' / 'novnc-1.6.0'
PUSH = 2.0          # seconds between state pushes
SNAP_TTL = 5.0      # a snapshot is taken again when older than this, and only when asked for
VNC_LINGER = 20.0   # a wayvnc nobody watches is stopped after this


def _alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except (ProcessLookupError, TypeError):
        return False
    except PermissionError:
        return True


def _session_state(sid):
    try:
        st = json.loads((config.CACHE / 'sessions' / sid / 'session.json').read_text())
    except (OSError, ValueError):
        return None
    if st.get('ended') or not (st.get('env') or {}).get('WAYLAND_DISPLAY'):
        return None
    if not _alive((st.get('pids') or {}).get('sway')):
        return None
    return st


def _wl_env(st):
    e = st['env']
    return dict(os.environ, WAYLAND_DISPLAY=e['WAYLAND_DISPLAY'], XDG_RUNTIME_DIR=e['XDG_RUNTIME_DIR'],
                SWAYSOCK=e.get('SWAYSOCK', ''))


# ---------------------------------------------------------------------- live views

class Vnc:
    """One view-only wayvnc per watched session, started when a page connects to it and
    stopped VNC_LINGER seconds after the last one leaves."""

    def __init__(self, base):
        self.base = base
        self.dir = config.CACHE / 'watch'
        self.dir.mkdir(parents=True, exist_ok=True)
        self.servers = {}       # sid -> dict(proc, port, clients, last)
        self.lock = threading.Lock()

    def _free_port(self):
        taken = {v['port'] for v in self.servers.values()}
        port = self.base
        while port in taken or _port_busy(port):
            port += 1
        return port

    def acquire(self, sid):
        st = _session_state(sid)
        if not st:
            return None
        with self.lock:
            v = self.servers.get(sid)
            if v and v['proc'].poll() is not None:
                self._stop(sid)
                v = None
            if not v:
                port = self._free_port()
                sock = self.dir / f'{sid}.ctl'
                sock.unlink(missing_ok=True)
                log = open(self.dir / f'{sid}.log', 'w')
                proc = subprocess.Popen(
                    ['wayvnc', '--disable-input', '--render-cursor', '--output', 'HEADLESS-1',
                     '--socket', str(sock), '127.0.0.1', str(port)],
                    env=_wl_env(st), stdout=log, stderr=subprocess.STDOUT,
                    stdin=subprocess.DEVNULL, start_new_session=True)
                v = self.servers[sid] = dict(proc=proc, port=port, clients=0, last=time.time())
                print(f'  + live view of {sid} (wayvnc on {port})', flush=True)
            v['clients'] += 1
            port = v['port']
        for _ in range(30):                 # until wayvnc listens
            if _port_busy(port):
                return port
            time.sleep(0.1)
        self.release(sid)
        return None

    def release(self, sid):
        with self.lock:
            v = self.servers.get(sid)
            if v:
                v['clients'] = max(0, v['clients'] - 1)
                v['last'] = time.time()

    def reap(self, force=False):
        with self.lock:
            for sid, v in list(self.servers.items()):
                if force or v['proc'].poll() is not None or not _session_state(sid) or \
                        (v['clients'] == 0 and time.time() - v['last'] > VNC_LINGER):
                    self._stop(sid)

    def _stop(self, sid):
        v = self.servers.pop(sid)
        if v['proc'].poll() is None:
            v['proc'].terminate()
            try:
                v['proc'].wait(3)
            except subprocess.TimeoutExpired:
                v['proc'].kill()
        (self.dir / f'{sid}.ctl').unlink(missing_ok=True)
        print(f'  - live view of {sid}', flush=True)

    def stop_all(self):
        self.reap(force=True)


class Snaps:
    """Each session's screen as a JPEG, in memory, taken with grim when asked for and the
    last one is older than SNAP_TTL -- so a session nobody looks at costs nothing. Keeps
    what each grim cost, for /api/snapcost."""

    def __init__(self):
        self.cache = {}         # sid -> (taken, jpeg bytes)
        self.locks = {}
        self.lock = threading.Lock()
        self.cost = []          # (sid, res, wall s, cpu s, bytes)

    def get(self, sid, ttl=SNAP_TTL):
        with self.lock:
            lk = self.locks.setdefault(sid, threading.Lock())
        with lk:
            hit = self.cache.get(sid)
            if hit and time.time() - hit[0] < ttl:
                return hit[1]
            st = _session_state(sid)
            if not st:
                return hit[1] if hit else None
            # full size: grim's scaling is done on the CPU and cost more than the JPEG
            # (1280x720: 10 ms at full size, 23 ms at -s 0.75, 105 ms at -s 0.5; measured)
            r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
            t0 = time.time()
            try:
                p = subprocess.run(['grim', '-o', 'HEADLESS-1', '-t', 'jpeg', '-q', '75', '-'],
                                   env=_wl_env(st), capture_output=True, timeout=10)
            except (OSError, subprocess.TimeoutExpired):
                return hit[1] if hit else None
            r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
            if p.returncode or not p.stdout:
                return hit[1] if hit else None
            cpu = (r1.ru_utime - r0.ru_utime) + (r1.ru_stime - r0.ru_stime)
            with self.lock:
                self.cost.append((sid, 'x'.join(map(str, st.get('res') or [])), round(time.time() - t0, 4),
                                  round(cpu, 4), len(p.stdout)))
                del self.cost[:-500]
            self.cache[sid] = (time.time(), p.stdout)
            return p.stdout

    def prune(self, live):
        with self.lock:
            for sid in list(self.cache):
                if sid not in live:
                    self.cache.pop(sid, None)


class Hub:
    """The state, computed once every PUSH seconds while a page is open, for every page."""

    def __init__(self, bench=None):
        self.bench = bench or dash.Bench()
        self.cond = threading.Condition()
        self.body = b'{}'
        self.state = {}
        self.seq = 0
        self.at = 0.0
        self.clients = 0
        self.wanted = time.time()

    def compute(self):
        try:
            s = self.bench.state()
        except Exception as e:          # a page that says why beats a silent one
            import traceback
            s = dict(error=f'{e}', trace=traceback.format_exc()[-2000:])
        _urls(s, self.bench)
        body = json.dumps(s).encode()
        with self.cond:
            self.state, self.body, self.at = s, body, time.time()
            self.seq += 1
            self.cond.notify_all()
        return s

    def run(self, vnc, snaps, stop):
        while not stop.is_set():
            if self.clients or time.time() - self.wanted < 10:
                s = self.compute()
                if s.get('ab_running'):
                    vnc.reap(force=True)        # nothing streams live during a perf A/B
                snaps.prune({v['sid'] for c in s.get('columns', []) for v in c.get('views', []) if v.get('sid')})
            vnc.reap()
            stop.wait(PUSH)


def _url(p, bench):
    if not p:
        return None
    p = Path(p)
    for prefix, base in (('/results/', bench.results), ('/baselines/', bench.cache / 'baselines')):
        try:
            return prefix + str(p.resolve().relative_to(Path(base).resolve()))
        except (ValueError, OSError):
            continue
    return None


def _urls(s, bench):
    for c in s.get('columns', []):
        for b in c.get('base') or []:
            b['heat'] = _url(b.get('heat'), bench)
        for d in c.get('dumps') or []:
            d['thumb'] = _url(d.get('thumb'), bench)
    for f in s.get('finished', []):
        f['link'] = _url(f.get('link'), bench)


# ---------------------------------------------------------------------- WebSocket -> RFB

def _recv_exact(sock, n):
    buf = b''
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            raise ConnectionError('closed')
        buf += chunk
    return buf


def _ws_send(sock, payload, opcode=0x2):
    n = len(payload)
    if n < 126:
        head = struct.pack('!BB', 0x80 | opcode, n)
    elif n < 1 << 16:
        head = struct.pack('!BBH', 0x80 | opcode, 126, n)
    else:
        head = struct.pack('!BBQ', 0x80 | opcode, 127, n)
    sock.sendall(head + payload)


def _shut(*socks):
    for s in socks:
        try:
            s.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass


def _bridge(client, port, stop_if):
    """Browser frames in, raw RFB out, and back. Both directions end when either does,
    or when stop_if() turns true (a perf A/B started)."""
    vnc = socket.create_connection(('127.0.0.1', port))
    lock = threading.Lock()
    done = threading.Event()

    def vnc_to_ws():
        try:
            while data := vnc.recv(65536):
                with lock:
                    _ws_send(client, data)
        except OSError:
            pass
        finally:
            done.set()
            _shut(client, vnc)

    def watchdog():
        while not done.wait(1.0):
            if stop_if():
                _shut(client, vnc)
                return

    threading.Thread(target=vnc_to_ws, daemon=True).start()
    threading.Thread(target=watchdog, daemon=True).start()
    try:
        while True:
            b0, b1 = _recv_exact(client, 2)
            op, n = b0 & 0x0f, b1 & 0x7f
            if n == 126:
                n, = struct.unpack('!H', _recv_exact(client, 2))
            elif n == 127:
                n, = struct.unpack('!Q', _recv_exact(client, 8))
            mask = _recv_exact(client, 4) if b1 & 0x80 else None
            data = _recv_exact(client, n) if n else b''
            if mask:
                data = bytes(c ^ mask[i % 4] for i, c in enumerate(data))
            if op == 0x8:
                break
            if op == 0x9:
                with lock:
                    _ws_send(client, data, 0xA)
            elif op in (0x0, 0x1, 0x2):
                vnc.sendall(data)
    except (OSError, ConnectionError):
        pass
    finally:
        done.set()
        _shut(vnc, client)
        vnc.close()


# ---------------------------------------------------------------------- HTTP

def _handler(hub, vnc, snaps):
    roots = {'/novnc/': NOVNC, '/results/': hub.bench.results, '/baselines/': hub.bench.cache / 'baselines'}

    class H(BaseHTTPRequestHandler):
        protocol_version = 'HTTP/1.1'      # a browser refuses an upgrade answered as 1.0

        def send(self, body, ctype, code=200, cache='no-store'):
            self.send_response(code)
            self.send_header('Content-Type', ctype)
            self.send_header('Cache-Control', cache)
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            path = unquote(self.path.split('?', 1)[0])
            if path.startswith('/ws/'):
                return self.websocket(path[4:])
            if path in ('/', '/index.html'):
                return self.send(PAGE.read_bytes(), 'text/html; charset=utf-8')
            if path == '/api/state':
                hub.wanted = time.time()
                if time.time() - hub.at > PUSH:
                    hub.compute()
                return self.send(hub.body, 'application/json')
            if path == '/api/snapcost':
                rows = list(snaps.cost)
                return self.send(json.dumps(dict(
                    snapshots=len(rows), rows=rows[-50:],
                    mean_wall=round(sum(r[2] for r in rows) / len(rows), 4) if rows else None,
                    mean_cpu=round(sum(r[3] for r in rows) / len(rows), 4) if rows else None)).encode(),
                    'application/json')
            if path == '/events':
                return self.events()
            if path.startswith('/snap/') and path.endswith('.jpg'):
                sid = path[6:-4]
                body = snaps.get(sid)
                if body is None:
                    return self.send_error(404)
                return self.send(body, 'image/jpeg')
            for prefix, base in roots.items():
                if path.startswith(prefix):
                    return self.static(Path(base), path[len(prefix):])
            self.send_error(404)

        def static(self, base, rel):
            try:
                p = (base / rel).resolve()
                p.relative_to(base.resolve())
            except (ValueError, OSError):
                return self.send_error(403)
            if p.is_dir():
                p = p / 'index.html'
            if not p.is_file():
                return self.send_error(404)
            ctype = mimetypes.guess_type(p.name)[0] or 'application/octet-stream'
            if p.suffix == '.js':
                ctype = 'text/javascript'
            self.send(p.read_bytes(), ctype, cache='max-age=3600' if base == NOVNC else 'no-store')

        def events(self):
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Cache-Control', 'no-store')
            self.send_header('Connection', 'keep-alive')
            self.end_headers()
            self.close_connection = True
            hub.clients += 1
            try:
                if not hub.state:
                    hub.compute()
                seq = -1
                while True:
                    with hub.cond:
                        if hub.seq == seq:
                            hub.cond.wait(PUSH * 3)
                        body, seq = hub.body, hub.seq
                    self.wfile.write(b'event: state\ndata: ' + body + b'\n\n')
                    self.wfile.flush()
            except (OSError, ConnectionError):
                pass
            finally:
                hub.clients -= 1

        def websocket(self, sid):
            key = self.headers.get('Sec-WebSocket-Key')
            if not key or 'websocket' not in self.headers.get('Upgrade', '').lower():
                return self.send_error(400)
            if hub.state.get('ab_running'):
                return self.send_error(409, 'a perf A/B is running: snapshots only')
            port = vnc.acquire(sid)
            if not port:
                return self.send_error(404)
            try:
                accept = base64.b64encode(hashlib.sha1(
                    (key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest()).decode()
                self.send_response(101)
                self.send_header('Upgrade', 'websocket')
                self.send_header('Connection', 'Upgrade')
                self.send_header('Sec-WebSocket-Accept', accept)
                if 'binary' in self.headers.get('Sec-WebSocket-Protocol', ''):
                    self.send_header('Sec-WebSocket-Protocol', 'binary')
                self.end_headers()
                self.wfile.flush()
                self.close_connection = True
                _bridge(self.connection, port, lambda: bool(hub.state.get('ab_running')))
            except OSError:
                pass
            finally:
                vnc.release(sid)

        def log_message(self, *a):
            pass
    return H


def _open_window(url, profile):
    for b in ('chromium', 'google-chrome-stable', 'google-chrome', 'brave'):
        if shutil.which(b):
            # its own profile, so it is its own window and process, closed with the watcher
            return subprocess.Popen([b, f'--app={url}', f'--user-data-dir={profile}',
                                     '--no-first-run', '--no-default-browser-check'],
                                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                    start_new_session=True)
    print(f'  no Chromium-family browser found; open {url} yourself', flush=True)
    return None


def serve(port, vnc_port, bench=None):
    """Start the server's threads; returns (httpd, stop event, vnc, snaps, hub)."""
    vnc, snaps, hub = Vnc(vnc_port), Snaps(), Hub(bench)
    httpd = ThreadingHTTPServer(('127.0.0.1', port), _handler(hub, vnc, snaps))
    httpd.daemon_threads = True
    stop = threading.Event()
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    threading.Thread(target=hub.run, args=(vnc, snaps, stop), daemon=True).start()
    return httpd, stop, vnc, snaps, hub


# ---------------------------------------------------------------------- desktop entry

APP_CLASS = 'chrome-127.0.0.1__-Default'   # Chromium's window class for --app=http://127.0.0.1:<any port>/
DESKTOP = (Path(os.environ.get('XDG_DATA_HOME') or Path.home() / '.local/share')
           / 'applications' / 'a2test-watch.desktop')
DESKTOP_LOG = config.CACHE / 'watch-desktop.log'


def _main_checkout():
    """The main worktree of the repository this runs from (every worktree shares the main
    checkout's .git as its common dir), else this checkout."""
    r = subprocess.run(['git', '-C', str(config.REPO), 'rev-parse', '--path-format=absolute',
                        '--git-common-dir'], capture_output=True, text=True)
    common = Path(r.stdout.strip()) if r.returncode == 0 else None
    if common and common.name == '.git' and (common.parent / 'a2test').exists():
        return common.parent
    return config.REPO


def install_desktop():
    repo = _main_checkout()
    a2test = repo / 'a2test'
    if any(c in str(a2test) for c in '"`$\\\n'):
        raise GameError(f'{a2test}: a character a desktop entry cannot quote simply; '
                        f'move the checkout')
    DESKTOP.parent.mkdir(parents=True, exist_ok=True)
    DESKTOP.write_text('\n'.join([
        '[Desktop Entry]',
        'Type=Application',
        'Name=Armada II Bench Watch',
        'GenericName=Test bench dashboard',
        f'Comment=a2test watch, from {repo}',
        f'Exec="{a2test}" watch --desktop',
        'Icon=utilities-system-monitor',
        'Terminal=false',
        'Categories=Development;',
        'Keywords=a2test;armada;bench;watch;dashboard;',
        f'StartupWMClass={APP_CLASS}',
        '']))
    if shutil.which('update-desktop-database'):
        subprocess.run(['update-desktop-database', str(DESKTOP.parent)],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(f'wrote {DESKTOP}\n  runs {a2test} watch --desktop (log: {DESKTOP_LOG})')
    return 0


def uninstall_desktop():
    if DESKTOP.exists():
        DESKTOP.unlink()
        print(f'removed {DESKTOP}')
    else:
        print(f'no {DESKTOP}')
    return 0


def _focus_running(url):
    """Bring a running watcher's window forward; open the page if there is none to find.
    hyprctl exits 0 whether or not it found the window, so its output is what is read."""
    if os.environ.get('HYPRLAND_INSTANCE_SIGNATURE') and shutil.which('hyprctl'):
        rx = APP_CLASS.replace('.', '\\.')            # the class must match whole
        lua = rx.replace('\\', '\\\\')                 # ... inside a Lua string
        for disp in ([f'hl.dsp.focus({{ window = "class:{lua}" }})'],   # the Lua dispatchers (0.55+)
                     ['focuswindow', f'class:^({rx})$']):               # the older syntax
            r = subprocess.run(['hyprctl', 'dispatch', *disp], capture_output=True, text=True)
            if r.returncode == 0 and r.stdout.strip() == 'ok':
                return
    subprocess.Popen(['xdg-open', url], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                     start_new_session=True)


def _desktop(argv, active_sessions):
    """Run from the desktop entry: no terminal, so output goes to DESKTOP_LOG and a failure
    to a notification; a second launch focuses the first watcher's window."""
    port = config.WATCH_PORT
    if '--port' in argv[:-1]:
        port = int(argv[argv.index('--port') + 1])
    if _port_busy(port):
        _focus_running(f'http://127.0.0.1:{port}/')
        return 0
    DESKTOP_LOG.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(DESKTOP_LOG, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644)
    os.dup2(fd, 1)
    os.dup2(fd, 2)
    os.close(fd)
    os.environ['NO_COLOR'] = '1'    # argparse colours its errors when FORCE_COLOR is set
    try:
        rc = main(argv, active_sessions)
    except GameError as e:
        print(f'a2test: {e}', flush=True)
        rc = 1
    except SystemExit as e:     # argparse
        rc = e.code if isinstance(e.code, int) else 1
    except Exception:
        import traceback
        traceback.print_exc()
        rc = 1
    sys.stdout.flush()
    if rc and shutil.which('notify-send'):
        tail = DESKTOP_LOG.read_text(errors='replace').strip().splitlines()[-3:]
        subprocess.run(['notify-send', '-u', 'critical', '-a', 'a2test watch',
                        f'a2test watch stopped ({rc})', '\n'.join(tail) or str(DESKTOP_LOG)],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return rc


def main(argv, active_sessions=None):
    if '--desktop' in argv:
        return _desktop([a for a in argv if a != '--desktop'], active_sessions)
    ap = argparse.ArgumentParser(prog='a2test watch')
    ap.add_argument('--port', type=int, default=config.WATCH_PORT, help='the page (default %(default)s)')
    ap.add_argument('--vnc-port', type=int, default=config.WATCH_PORT + 1,
                    help='first loopback port for the wayvnc servers (default %(default)s)')
    ap.add_argument('--no-open', action='store_true', help='serve the page, open no window')
    ap.add_argument('--desktop', action='store_true',
                    help='as the desktop entry runs it: output to a log, failures as a notification, '
                         'and a watcher already running is focused instead')
    ap.add_argument('--install-desktop', action='store_true',
                    help="write a desktop entry that runs the main checkout's watcher, and exit")
    ap.add_argument('--uninstall-desktop', action='store_true', help='remove that entry, and exit')
    a = ap.parse_args(argv)
    if a.install_desktop:
        return install_desktop()
    if a.uninstall_desktop:
        return uninstall_desktop()
    if not shutil.which('wayvnc'):
        print('  wayvnc is not installed (pacman -S wayvnc): snapshots only, no live view', flush=True)
    url = f'http://127.0.0.1:{a.port}/'
    try:
        httpd, stop, vnc, snaps, hub = serve(a.port, a.vnc_port)
    except OSError:
        raise GameError(f'port {a.port} is busy -- is a watcher already running? ({url}); '
                        f'else --port N')
    print(f'the bench dashboard: {url}  (Ctrl-C stops)', flush=True)
    browser = None
    try:
        if not a.no_open:
            browser = _open_window(url, vnc.dir / 'browser')
        while True:
            time.sleep(1)
            if browser and browser.poll() is not None:
                print('window closed', flush=True)
                break
    except KeyboardInterrupt:
        pass
    finally:
        stop.set()
        httpd.shutdown()
        vnc.stop_all()
        if browser and browser.poll() is None:
            browser.terminate()
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
