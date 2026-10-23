"""The stock game, kept: `a2mod stock` run once on a reflink clone of the user's install,
and every stock session cloned from that instead of from the install.

A session that starts from stock (`--mod stock`, `--install PATH`, `--assets none`) used to
clone the user's install and run `a2mod stock` on the clone: 12 s of hashing and copying
1879 modded files, the same work with the same result every time. The result depends
only on the install's files and on the `a2mod` that ran, so it is made once, kept under
`~/.cache/a2test/stock/<key>/game` (the bench's own cache, never a checkout: it is the
game's own files), and each session takes a reflink copy of it (0.7 s, no space).

THE KEY is a fingerprint of the install as it stands -- every path under it with its
type, mode, size, inode, mtime and ctime -- and of the `a2mod` script that made the
entry, with KEY_VERSION for how this module makes it. ctime cannot be set from user
space, so anything that writes, renames, moves or chmods a file in the install (the user
playing and the plugins writing their logs, `./install`, `a2mod`, Heroic or DXVK
updating a DLL) gives a new key, and the next stock session builds a new entry. A false
miss costs one build; a false hit is the thing the key is made to rule out. The key is
taken again after the build, and an install that changed while it was being copied is
not kept.

CONCURRENCY. Every check, build, prune and copy runs under one flock (`stock/.lock`):
a build goes into `.build-<key>-<pid>` and is renamed into place, so a half-built entry
is never seen, and a prune can never delete an entry another session is copying. A
session that finds another building waits for it (about 12 s) and then copies its entry.

`A2TEST_STOCK_CACHE=0` turns it off (the clone and `a2mod stock` as before);
`a2test stock-cache status | rebuild | drop` shows, rebuilds or removes it.
"""
import fcntl
import hashlib
import json
import os
import shutil
import subprocess
import time
from contextlib import contextmanager
from pathlib import Path

from . import config

ROOT = config.CACHE / 'stock'
LOCK = ROOT / '.lock'
KEY_VERSION = 1      # bump when how an entry is made changes
KEEP = 3             # entries kept, the most recently used first


class StockCacheError(Exception):
    pass


def enabled():
    return os.environ.get('A2TEST_STOCK_CACHE', '1').strip().lower() not in ('0', 'no', 'off', 'false')


def a2mod_path():
    return config.REPO / 'a2mod'


def fingerprint(game=None, a2mod=None):
    """The key: the install's tree by lstat, and the a2mod that makes the entry."""
    game = Path(game or config.GAME)
    a2mod = Path(a2mod or a2mod_path())
    h = hashlib.sha256()
    h.update(f'a2test stock cache v{KEY_VERSION}\n'.encode())
    h.update(hashlib.sha256(a2mod.read_bytes()).hexdigest().encode() + b'\n')
    n = 0
    for dirpath, dirs, files in os.walk(game):
        dirs.sort()
        for name in sorted(dirs + files):
            p = os.path.join(dirpath, name)
            try:
                s = os.lstat(p)
            except FileNotFoundError:
                continue
            rel = os.path.relpath(p, game)
            h.update(f'{rel}\0{s.st_mode:o}\0{s.st_size}\0{s.st_ino}\0'
                     f'{s.st_mtime_ns}\0{s.st_ctime_ns}\n'.encode(errors='surrogateescape'))
            n += 1
    s = os.stat(game)
    h.update(f'.\0{s.st_ino}\0{s.st_mtime_ns}\0{s.st_ctime_ns}\0{n}\n'.encode())
    return h.hexdigest()[:20]


@contextmanager
def locked():
    ROOT.mkdir(parents=True, exist_ok=True)
    with open(LOCK, 'a') as fh:
        fcntl.flock(fh, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(fh, fcntl.LOCK_UN)


def _reflink(src, dst):
    r = subprocess.run(['cp', '-a', '--reflink=always', str(src), str(dst)],
                       capture_output=True, text=True)
    if r.returncode:
        raise StockCacheError(f'reflink copy of {src} failed (must be btrfs, same '
                              f'filesystem as {config.CACHE}): {r.stderr.strip()}')


def _rmtree(p):
    shutil.rmtree(p, ignore_errors=True)


def entries():
    """Every complete entry, most recently used first."""
    if not ROOT.is_dir():
        return []
    out = []
    for d in ROOT.iterdir():
        m = d / 'manifest.json'
        if d.name.startswith('.') or not m.is_file():
            continue
        try:
            info = json.loads(m.read_text())
        except (OSError, ValueError):
            continue
        info['dir'] = str(d)
        info['used'] = (d / 'used').stat().st_mtime if (d / 'used').exists() else m.stat().st_mtime
        out.append(info)
    return sorted(out, key=lambda e: -e['used'])


def _build(key):
    """`a2mod stock` on a fresh reflink clone of the install, renamed into place as
    ROOT/<key>. Called under the lock."""
    tmp = ROOT / f'.build-{key}-{os.getpid()}'
    _rmtree(tmp)
    tmp.mkdir(parents=True)
    t = time.time()
    try:
        _reflink(config.GAME, tmp / 'game')
        env = dict(os.environ, A2_GAME=str(tmp / 'game'), A2_GAME_DIR=str(tmp / 'game'))
        r = subprocess.run([str(a2mod_path()), 'stock'], env=env, cwd=str(config.REPO),
                           capture_output=True, text=True)
        out = (r.stdout + r.stderr).replace(str(tmp / 'game'), '<clone>')
        # An install with nothing of ours in it is stock already (a2mod says so and exits 1).
        if r.returncode and 'nothing to switch' not in out:
            raise StockCacheError(f'a2mod stock failed on the cache clone (exit {r.returncode}):\n{out}')
        after = fingerprint()
        if after != key:
            raise StockCacheError('the install changed while the stock cache was being built '
                                  f'({key} -> {after}); not kept')
        (tmp / 'a2mod.log').write_text(out)
        (tmp / 'manifest.json').write_text(json.dumps(dict(
            key=key, game=str(config.GAME), a2mod=str(a2mod_path()),
            a2mod_sha256=hashlib.sha256(a2mod_path().read_bytes()).hexdigest(),
            key_version=KEY_VERSION, built=time.strftime('%Y-%m-%d %H:%M:%S'),
            seconds=round(time.time() - t, 1)), indent=1))
        dst = ROOT / key
        _rmtree(dst)
        os.rename(tmp, dst)
        return dst
    finally:
        _rmtree(tmp)


def _prune(keep_dir):
    """Keep the KEEP most recently used entries (and the one in hand); drop stale builds
    of processes that are gone. Called under the lock."""
    for i, e in enumerate(entries()):
        if i >= KEEP and e['dir'] != str(keep_dir):
            _rmtree(e['dir'])
    for d in ROOT.glob('.build-*'):
        pid = d.name.rsplit('-', 1)[-1]
        if not pid.isdigit() or not Path(f'/proc/{pid}').exists():
            _rmtree(d)


def clone_stock(dst, rebuild=False):
    """Reflink-copy the stock game into `dst`, building the entry first if the install
    (or a2mod) changed since. Returns a dict: key, built (True if built now), seconds,
    a2mod log. The real install is only ever read."""
    with locked():
        t = time.time()
        key = fingerprint()
        entry = ROOT / key
        built = False
        if rebuild or not (entry / 'manifest.json').is_file():
            entry = _build(key)
            built = True
        build_s = round(time.time() - t, 1)
        if dst is not None:
            _reflink(entry / 'game', dst)
        (entry / 'used').touch()
        _prune(entry)
        info = json.loads((entry / 'manifest.json').read_text())
        info.update(built_now=built, build_seconds=build_s if built else 0,
                    log=(entry / 'a2mod.log').read_text() if (entry / 'a2mod.log').exists() else '')
        return info


def drop():
    with locked():
        n = len(entries())
        for d in ROOT.iterdir():
            if d.name != '.lock':
                _rmtree(d)
        return n


def main(argv):
    sub = argv[0] if argv else 'status'
    if sub == 'status':
        cur = fingerprint()
        es = entries()
        print(f'stock cache: {ROOT}  ({"on" if enabled() else "OFF: A2TEST_STOCK_CACHE=0"})')
        print(f'install now: {config.GAME}  key {cur}')
        if not es:
            print('  no entries (the next stock session builds one)')
        for e in es:
            mark = 'current' if e['key'] == cur else 'stale  '
            print(f"  {mark} {e['key']}  built {e['built']} in {e.get('seconds', '?')} s, "
                  f"last used {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(e['used']))}")
        return 0
    if sub == 'rebuild':
        info = clone_stock(None, rebuild=True)
        print(f"built {info['key']} in {info['build_seconds']} s")
        print(info['log'].rstrip())
        return 0
    if sub == 'drop':
        print(f'dropped {drop()} entries from {ROOT}')
        return 0
    print('a2test stock-cache [status | rebuild | drop]')
    return 2
