"""a2test watch -- every live session, view-only, tiled in one window.

Sessions do not need to have been started with --vnc. The watcher starts its own
wayvnc on each live session's headless output -- view-only, cursor drawn in, on a
loopback port -- and serves one page that tiles them with noVNC, bridging each tile's
WebSocket (/ws/<session>) to its wayvnc. Sessions that start later appear, ended ones
drop out. Ctrl-C stops the servers it started and nothing else.

The bridge is here rather than `wayvnc --websocket` because that segfaults on the first
client (wayvnc 0.10.1 / neatvnc 1.0.1, a call through a null pointer -- measured).
"""
import argparse
import base64
import hashlib
import json
import os
import shutil
import socket
import struct
import subprocess
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from . import config
from .session import GameError, _port_busy

PAGE = Path(__file__).with_name('watch.html')
POLL = 2.0


def _alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except (ProcessLookupError, TypeError):
        return False
    except PermissionError:
        return True


class Watcher:
    def __init__(self, active_sessions, vnc_base):
        self.active_sessions = active_sessions
        self.vnc_base = vnc_base
        self.dir = config.CACHE / 'watch'
        self.dir.mkdir(parents=True, exist_ok=True)
        self.servers = {}       # session id -> dict(proc, port, info)
        self.lock = threading.Lock()

    def _free_port(self):
        taken = {v['port'] for v in self.servers.values()}
        port = self.vnc_base
        while port in taken or _port_busy(port):
            port += 1
        return port

    def _info(self, s):
        try:
            where = str(s.dir.relative_to(config.RESULTS))
        except ValueError:
            where = str(s.dir)
        return dict(id=s.s['id'], where=where, label=s.s.get('label', ''), mod=s.s['mod'],
                    res=list(s.res), game=s.running())

    def poll(self):
        live = {}
        for s in self.active_sessions():
            env = s.s.get('env') or {}
            if env.get('WAYLAND_DISPLAY') and _alive(s.s.get('pids', {}).get('sway')):
                live[s.s['id']] = s
        with self.lock:
            for sid in list(self.servers):
                if sid not in live or self.servers[sid]['proc'].poll() is not None:
                    self._stop(sid)
            for sid, s in live.items():
                if sid in self.servers:
                    self.servers[sid]['info'] = self._info(s)
                    continue
                port = self._free_port()
                sock = self.dir / f'{sid}.ctl'
                sock.unlink(missing_ok=True)
                log = open(self.dir / f'{sid}.log', 'w')
                # -S: a session started with --vnc already has a wayvnc on the default
                # control socket in its runtime dir
                proc = subprocess.Popen(
                    ['wayvnc', '--disable-input', '--render-cursor', '--output', 'HEADLESS-1',
                     '--socket', str(sock), '127.0.0.1', str(port)],
                    env=s.wl_env(), stdout=log, stderr=subprocess.STDOUT,
                    stdin=subprocess.DEVNULL, start_new_session=True)
                self.servers[sid] = dict(proc=proc, port=port, info=self._info(s))
                print(f'  + {sid}  {self.servers[sid]["info"]["where"]}', flush=True)

    def _stop(self, sid):
        v = self.servers.pop(sid)
        if v['proc'].poll() is None:
            v['proc'].terminate()
            try:
                v['proc'].wait(3)
            except subprocess.TimeoutExpired:
                v['proc'].kill()
        (self.dir / f'{sid}.ctl').unlink(missing_ok=True)
        print(f'  - {sid}', flush=True)

    def stop_all(self):
        with self.lock:
            for sid in list(self.servers):
                self._stop(sid)

    def listing(self):
        with self.lock:
            return [dict(v['info'], port=v['port']) for v in self.servers.values()]

    def port_of(self, sid):
        with self.lock:
            v = self.servers.get(sid)
            return v and v['port']


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


def _bridge(client, port):
    """Browser frames in, raw RFB out, and back. Both directions end when either does."""
    vnc = socket.create_connection(('127.0.0.1', port))
    lock = threading.Lock()

    def vnc_to_ws():
        try:
            while data := vnc.recv(65536):
                with lock:
                    _ws_send(client, data)
        except OSError:
            pass
        finally:
            _shut(client, vnc)

    threading.Thread(target=vnc_to_ws, daemon=True).start()
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
        _shut(vnc, client)
        vnc.close()


def _handler(watcher):
    class H(BaseHTTPRequestHandler):
        protocol_version = 'HTTP/1.1'      # a browser refuses an upgrade answered as 1.0

        def do_GET(self):
            path = self.path.split('?', 1)[0]
            if path.startswith('/ws/'):
                return self.websocket(path[4:])
            if path == '/sessions.json':
                body, ctype = json.dumps(watcher.listing()).encode(), 'application/json'
            elif path in ('/', '/index.html'):
                body, ctype = PAGE.read_bytes(), 'text/html; charset=utf-8'
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header('Content-Type', ctype)
            self.send_header('Cache-Control', 'no-store')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def websocket(self, sid):
            port = watcher.port_of(sid)
            key = self.headers.get('Sec-WebSocket-Key')
            if not port or not key or 'websocket' not in self.headers.get('Upgrade', '').lower():
                self.send_error(404 if not port else 400)
                return
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
            try:
                _bridge(self.connection, port)
            except OSError:
                pass

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


def main(argv, active_sessions):
    ap = argparse.ArgumentParser(prog='a2test watch')
    ap.add_argument('--port', type=int, default=config.WATCH_PORT, help='the page (default %(default)s)')
    ap.add_argument('--vnc-port', type=int, default=config.WATCH_PORT + 1,
                    help='first loopback port for the wayvnc servers (default %(default)s)')
    ap.add_argument('--no-open', action='store_true', help='serve the page, open no window')
    a = ap.parse_args(argv)
    if not shutil.which('wayvnc'):
        raise GameError('watch needs wayvnc (pacman -S wayvnc)')
    url = f'http://127.0.0.1:{a.port}/'
    w = Watcher(active_sessions, a.vnc_port)
    try:
        httpd = ThreadingHTTPServer(('127.0.0.1', a.port), _handler(w))
    except OSError:
        raise GameError(f'port {a.port} is busy -- is a watcher already running? ({url}); '
                        f'else --port N')
    httpd.daemon_threads = True
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    print(f'watching a2test sessions: {url}  (Ctrl-C stops)', flush=True)
    browser = None
    try:
        w.poll()
        if not a.no_open:
            browser = _open_window(url, w.dir / 'browser')
        while True:
            time.sleep(POLL)
            w.poll()
            if browser and browser.poll() is not None:
                print('window closed', flush=True)
                break
    except KeyboardInterrupt:
        pass
    finally:
        httpd.shutdown()
        w.stop_all()
        if browser and browser.poll() is None:
            browser.terminate()
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:], lambda: []))
