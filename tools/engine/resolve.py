#!/usr/bin/env python3
"""The resolver (phase 8 contract P4) in Python, offline: the same algorithm as
crates/a2engine (src/engine.rs) and csdk/a2engine.h, proven on the same fixtures
(tools/engine/fixtures/), and what `a2test syms` reports with.

    resolve.py --exe PATH [--plugin P] [--build B] [--test-ini FILE] [--base VA]
               [--profiles DIR ...]

prints, per plugin table, the transcript a plugin would log at init, then each hook's
`ready`. The exe is only read. `--build` forces that build's profile onto the exe
(the wrong-profile test); `--base` maps it somewhere else than its ImageBase.
"""
import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe  # noqa: E402
import profiles  # noqa: E402

NOTE_MAX = 120
# `{exe}`: the matched build's executable (build.toml `exe`), else the module's name.
PLUGIN_NOTE = 'This {exe} is a build Refit does not know: it runs stock'
TEST_WHY = 'test mismatch (A2EngineTest.ini)'


def test_globs(text):
    """`[Test] Mismatch=<glob>[,<glob>]` of an A2EngineTest.ini's text."""
    if not text:
        return []
    sect, val = '', None
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line[0] in ';#':
            continue
        if line.startswith('[') and line.endswith(']'):
            sect = line[1:-1].strip().lower()
            continue
        if sect == 'test' and '=' in line:
            k, v = line.split('=', 1)
            if k.strip().lower() == 'mismatch':
                val = v
    if val is None:
        return []
    return [g.strip() for g in val.split(',') if g.strip()]


def note_text(s):
    return s[:NOTE_MAX]


class Engine:
    def __init__(self, table, exe, test_ini=None, log=None, force_build=None):
        self.t = table
        self.exe = exe
        self.lines = []
        self.log = log or self.lines.append
        n_syms = len(table.sym_ids)
        self.state = {}          # hook index -> (ok, symbol, why)
        self.ready_syms = [False] * n_syms
        self.addrs = [0] * n_syms
        self.warned = [False] * n_syms
        self.bi = None
        for i, b in enumerate(table.builds):
            pe_ = b['build'].pe
            if force_build is not None:
                hit = b['build'].name == force_build
            else:
                hit = (pe_.get('timestamp'), pe_.get('image_size'), pe_.get('sections')) == exe.identity()
            if hit:
                self.bi = i
                break
        # The executable's name for the log and the notes: the build's, else the module's.
        self.exe_name = (self.build['build'].exe if self.bi is not None else None) or exe.name or 'exe'
        self.log(exe.line(self.exe_name))
        if self.bi is not None:
            b = self.build
            self.delta = (exe.base - b['build'].pe.get('image_base', 0)) & 0xffffffff
            self.log(f'engine: build {b["build"].key}')
        else:
            self.delta = 0
            self.log('engine: unknown build')
        self.globs = test_globs(test_ini)
        m = n_syms
        if self.bi is None:
            n = sum(1 for k in range(m) if table.kind(k) == 'scan')
            h = 0
        else:
            blocked = self.build['blocked']
            n = 0
            for k, e in enumerate(self.build['entries']):
                if e is None:
                    continue
                ub = [f'{table.plugin}.{u}' for u in (e.get('used_by') or [])]
                if ub and all(u in blocked for u in ub):
                    continue
                n += 1
            h = len(blocked)
        self.log(f'engine: symbols: {n}/{m} resolved; {h} hooks blocked')
        if self.globs:
            self.log(f'engine: TEST: A2EngineTest.ini forces a mismatch for {", ".join(self.globs)}')

    @property
    def build(self):
        return self.t.builds[self.bi] if self.bi is not None else None

    def build_key(self):
        return self.build['build'].key if self.bi is not None else None

    def entries(self):
        """This build's entries; on an unknown build the root's (only scans are used)."""
        return (self.build or self.t.builds[0])['entries']

    def sym_index(self, sid):
        return self.t.sym_ids.index(sid)

    def resolved(self, k):
        """(address or value, None) or (0, (symbol, why)) for entry k, unchecked."""
        e = self.entries()[k]
        if e is None:
            return 0, (self.t.defs[self.t.sym_ids[k]].get('symbol', ''), f'has no address in {(self.build or self.t.builds[0])["build"].name}')
        kind = e['kind']
        if kind == 'field':
            return e['offset'], None
        if kind == 'value':
            return e['value'], None
        if kind == 'scan':
            return self.scan(e)
        if kind == 'slot':
            vk = self.sym_index(e['vtable'])
            vt, err = self.resolved(vk)
            if err:
                return 0, err
            return (vt + 4 * e['index']) & 0xffffffff, None
        return (e['addr'] + self.delta) & 0xffffffff, None

    def scan(self, e):
        pat, mask = profiles.hexbytes(e['pattern'], e['id'], pattern=True)
        sec = self.exe.section(e['section'])
        hits = []
        if sec is not None and pat:
            va, size = sec
            data = self.exe.read(va, size) or b''
            first = next((i for i, x in enumerate(mask) if x), None)
            for i in range(0, len(data) - len(pat) + 1):
                if first is not None and data[i + first] != pat[first]:
                    continue
                if all(not mask[j] or data[i + j] == pat[j] for j in range(len(pat))):
                    hits.append(va + i)
        if len(hits) != e['count']:
            return 0, (e.get('symbol', ''), f'is found {len(hits)} times, not {e["count"]}')
        return (hits[0] if hits else 0), None

    def in_code(self, a):
        return self.exe.code[0] <= a < self.exe.code[1]

    def check(self, k, chained):
        """None if entry k passes its check in memory, else (symbol, why)."""
        a, err = self.resolved(k)
        if err:
            return err, a
        e = self.entries()[k]
        kind, sym = e['kind'], e.get('symbol', '')
        if kind in ('fn', 'label', 'patch') and e.get('check'):
            want, _ = profiles.hexbytes(e['check'], e['id'])
            if self.exe.read(a, len(want)) != want:
                return (sym, 'is not stock (bytes differ)'), a
        elif kind in ('call', 'jmp'):
            tk = self.sym_index(e['target'])
            ta, terr = self.resolved(tk)
            if terr:
                return terr, a
            b = self.exe.read(a, 5)
            if b is None or b[0] != (0xe8 if kind == 'call' else 0xe9):
                return (sym, 'calls something else'), a
            dest = (a + 5 + struct.unpack_from('<i', b, 1)[0]) & 0xffffffff
            if dest != ta:
                if e.get('chain') and not self.in_code(dest):
                    chained.append(f'{sym} at 0x{a:x} calls 0x{dest:x} (chained)')
                else:
                    return (sym, 'calls something else'), a
        elif kind == 'slot':
            b = self.exe.read(a, 4)
            if b is None:
                return (sym, 'calls something else'), a
            v = struct.unpack('<I', b)[0]
            if e['target'] != 'any':
                ta, terr = self.resolved(self.sym_index(e['target']))
                if terr:
                    return terr, a
                if v != ta:
                    if e.get('chain') and not self.in_code(v):
                        chained.append(f'{sym} at 0x{a:x} holds 0x{v:x} (chained)')
                    else:
                        return (sym, 'calls something else'), a
        return None, a

    def ready(self, h):
        """True if hook h (index or id) may run; else False, its stand-down logged once."""
        if isinstance(h, str):
            h = self.t.hook_ids.index(h)
        if h in self.state:
            return self.state[h][0]
        hid = self.t.hook_ids[h]
        ks = self.t.hook_syms(self.t.hooks[h])
        sym0 = self.t.defs[self.t.sym_ids[ks[0]]].get('symbol', '') if ks else ''
        fail = None
        if self.bi is None and not self.t.file.hooks[self.t.hooks[h]].get('unknown') == 'run':
            fail = (sym0, 'unknown build')
        elif self.bi is not None and hid in self.build['blocked']:
            fail = self.build['blocked'][hid]
        elif any(profiles.glob_match(g, hid) for g in self.globs):
            fail = (sym0, TEST_WHY)
        chained, got = [], {}
        if fail is None:
            for k in ks:
                err, a = self.check(k, chained)
                if err and fail is None:
                    fail = err
                got[k] = a
        if fail is not None:
            self.log(f'engine: {hid} stands down: {fail[0]} {fail[1]}')
            self.state[h] = (False, fail[0], fail[1])
            return False
        for c in chained:
            self.log(f'engine: {hid}: {c}')
        for k, a in got.items():
            self.ready_syms[k] = True
            self.addrs[k] = a
        self.state[h] = (True, '', '')
        return True

    def addr(self, k):
        if isinstance(k, str):
            k = self.sym_index(k)
        if self.ready_syms[k]:
            return self.addrs[k]
        if not self.warned[k]:
            self.warned[k] = True
            self.log(f'engine: {self.t.sym_ids[k]} read before its hook was ready')
        return 0

    def field(self, k):
        """A field's offset or a value in this build (the root build's on an unknown build;
        0 where the build has none): crates/a2engine's `Engine::field`/`value`."""
        if isinstance(k, str):
            k = self.sym_index(k)
        e = self.entries()[k]
        if e is None:
            return 0
        return e['offset'] if e['kind'] == 'field' else e['value']

    def note(self, h):
        if isinstance(h, str):
            h = self.t.hook_ids.index(h)
        st = self.state.get(h)
        if st is None or st[0]:
            return None
        sym, why = st[1], st[2]
        if why == 'unknown build':
            return note_text(PLUGIN_NOTE.format(exe=self.exe_name))
        if why == TEST_WHY:
            return note_text(f'Stands down: {sym}: {why}')
        if why.startswith('is changed by '):
            by = why[len('is changed by '):].split(' ')[0]
            return note_text(f'Stands down: {sym} is changed by {by}, not yet verified')
        if why.startswith('is off in '):
            return note_text(f'Stands down: {why[3:]}')
        return note_text(f'Stands down: {sym} is not stock in this {self.exe_name}')

    def note_for(self, section, key):
        row = f'{section}.{key}'
        for h, name in enumerate(self.t.hooks):
            if row in (self.t.file.hooks[name].get('rows') or []):
                n = self.note(h)
                if n:
                    return n
        return self.note_plugin() if self.bi is None else None

    def note_plugin(self):
        return note_text(PLUGIN_NOTE.format(exe=self.exe_name)) if self.bi is None else None


def transcript(table, exe, test_ini=None, addr=(), note_for=(), force_build=None, field=()):
    """The fixture transcript (tools/engine/README.md, "Fixtures")."""
    eng = Engine(table, exe, test_ini, force_build=force_build)
    out = eng.lines
    for h, hid in enumerate(table.hook_ids):
        ok = eng.ready(h)
        out.append(f'ready {hid} {"ok" if ok else "fail"}')
    for sid in addr:
        v = eng.addr(sid)
        out.append(f'addr {sid} 0x{v & 0xffffffff:x}')
    for sid in field:
        v = eng.field(sid)
        out.append(f'field {sid} 0x{v & 0xffffffff:x}')
    for h, hid in enumerate(table.hook_ids):
        n = eng.note(h)
        if n is not None:
            out.append(f'note {hid} {n}')
    for row in note_for:
        s, k = row.split('.', 1)
        out.append(f'note_for {row} {eng.note_for(s, k) or "-"}')
    out.append(f'note_plugin {eng.note_plugin() or "-"}')
    return out


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--exe', required=True)
    ap.add_argument('--plugin')
    ap.add_argument('--build')
    ap.add_argument('--test-ini')
    ap.add_argument('--base', type=lambda s: int(s, 0))
    ap.add_argument('--profiles', nargs='*')
    a = ap.parse_args(argv)
    with open(a.exe, 'rb') as f:
        exe = pe.Exe(f.read(), a.base, name=os.path.basename(a.exe))
    ini = open(a.test_ini).read() if a.test_ini else None
    games = profiles.load_game_sets(a.profiles if a.profiles else None)
    for t in profiles.tables(games):
        if a.plugin and t.plugin != a.plugin:
            continue
        print(f'# {t.file.rel()}')
        for line in transcript(t, exe, ini, force_build=a.build):
            print(line)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
