#!/usr/bin/env python3
"""The resolver fixtures (tools/engine/README.md, "Fixtures"): one synthetic PE image
and the profile set it is resolved against per `fixtures/NN-name.toml`, and the
transcript every resolver must print for each of the set's tables. Three
implementations run them: Python (resolve.py), C (csdk/a2engine.h, through ctest.c)
and Rust (crates/a2engine, `cargo test --features fixtures`).

    fixtures.py run [py|c|rs ...]   run them (default: all three); exit 1 on a difference
    fixtures.py bless               write Python's transcripts into the fixtures' [expect]
                                    (review the diff: the transcript is the spec)
    fixtures.py rust-module OUT.rs  write the Rust test module (what `run rs` builds)

A fixture: `set` (a directory of fixtures/sets/), `[image]` (mkpe.py), optional `base`
(where the image is mapped; default its ImageBase), `test_ini` (an A2EngineTest.ini's
text), `addr` (symbol ids queried after the readies, those of each table), `note_for`
(`Section.Key` rows), `field` (field and value ids whose per-build offset or value is
queried after the addrs), and `[expect]` with one transcript per plugin.
"""
import glob
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen  # noqa: E402
import mkpe  # noqa: E402
import pe  # noqa: E402
import profiles  # noqa: E402
import resolve  # noqa: E402

ROOT = profiles.ROOT
FIX = os.path.join(HERE, 'fixtures')


def fixtures():
    out = []
    for p in sorted(glob.glob(os.path.join(FIX, '[0-9][0-9]-*.toml'))):
        with open(p, 'rb') as f:
            d = tomllib.load(f)
        name = os.path.basename(p)[:-5]
        games = profiles.load_game_sets([os.path.join(FIX, 'sets', d['set'])])
        out.append((name, p, d, profiles.tables(games)))
    return out


def queries(d, t):
    return [a for a in d.get('addr', []) if a in t.sym_ids], list(d.get('note_for', []))


def field_queries(d, t):
    return [a for a in d.get('field', []) if a in t.sym_ids]


def py_transcript(d, t):
    exe = pe.Exe(mkpe.build(d['image']), d.get('base'))
    addr, nf = queries(d, t)
    return resolve.transcript(t, exe, d.get('test_ini'), addr, nf, field=field_queries(d, t))


def expected(d, t):
    return list((d.get('expect') or {}).get(t.plugin, []))


def content_expect(d, t):
    """The content set the fixture expects of this table ('none' for no set), or None
    when the fixture does not say."""
    return (d.get('content') or {}).get(t.plugin)


def py_content(d, t):
    exe = pe.Exe(mkpe.build(d['image']), d.get('base'))
    eng = resolve.Engine(t, exe, d.get('test_ini'))
    if eng.bi is None:
        return 'none'
    return eng.build['build'].content_for(t.fname) or 'none'


def report(impl, name, plugin, want, got):
    if want == got:
        return True
    print(f'FAIL {impl} {name} {plugin}:')
    for i in range(max(len(want), len(got))):
        w = want[i] if i < len(want) else '<none>'
        g = got[i] if i < len(got) else '<none>'
        if w != g:
            print(f'  line {i + 1}:\n    want {w}\n    got  {g}')
            break
    return False


def run_py(fx):
    ok = True
    for name, _, d, ts in fx:
        for t in ts:
            ok &= report('py', name, t.plugin, expected(d, t), py_transcript(d, t))
            want = content_expect(d, t)
            if want is not None:
                ok &= report('py content', name, t.plugin, [want], [py_content(d, t)])
    return ok


def cc():
    for c in ('cc', 'clang', 'gcc'):
        if shutil.which(c):
            return c
    return None


def run_c(fx, tmp):
    c = cc()
    if c is None:
        print('c: no C compiler, the C resolver was NOT tested')
        return True
    ok = True
    for name, _, d, ts in fx:
        img = os.path.join(tmp, f'{name}.exe')
        with open(img, 'wb') as f:
            f.write(mkpe.build(d['image']))
        ini = '-'
        if d.get('test_ini') is not None:
            ini = os.path.join(tmp, f'{name}.ini')
            with open(ini, 'w') as f:
                f.write(d['test_ini'])
        for t in ts:
            hdr = os.path.join(tmp, f'{name}_{t.plugin}.h')
            with open(hdr, 'w') as f:
                f.write(gen.emit(t, 'c'))
            exe = os.path.join(tmp, f'ctest_{name}_{t.plugin}')
            r = subprocess.run([c, '-std=c99', '-Wall', '-Werror', '-DA2E_HOST', '-I', os.path.join(ROOT, 'csdk'),
                                f'-DA2E_TABLE_H="{hdr}"', f'-DA2E_TABLE={gen.upper(t.plugin)}_ENGINE',
                                os.path.join(HERE, 'ctest.c'), '-o', exe], capture_output=True, text=True)
            if r.returncode:
                print(f'FAIL c {name} {t.plugin}: does not compile\n{r.stderr[-2000:]}')
                ok = False
                continue
            addr, nf = queries(d, t)
            base = d.get('base', d['image']['image_base'])
            args = [exe, img, ini, f'{base:x}']
            for a in addr:
                args += ['addr', a]
            for a in field_queries(d, t):
                args += ['field', a]
            for n in nf:
                args += ['note_for', n]
            r = subprocess.run(args, capture_output=True, text=True)
            got = r.stdout.splitlines()
            if r.returncode:
                got.append(f'<exit {r.returncode}>')
            ok &= report('c', name, t.plugin, expected(d, t), got)
    return ok


def rs_str(s):
    return '"' + s.replace('\\', '\\\\').replace('"', '\\"') + '"'


def rust_module(fx, tmp):
    o = ['// generated by tools/engine/fixtures.py; do not edit', '']
    cases = []
    for name, _, d, ts in fx:
        img = os.path.join(tmp, f'{name}.exe')
        with open(img, 'wb') as f:
            f.write(mkpe.build(d['image']))
        for t in ts:
            m = f'f{name[:2]}_{t.plugin}'
            o.append(f'#[allow(dead_code)]\npub mod {m} {{')
            o.append(gen.emit(t, 'rust').replace('//! ', '// '))
            o.append('}')
            addr, nf = queries(d, t)
            base = d.get('base', d['image']['image_base'])
            ini = 'None' if d.get('test_ini') is None else f'Some({rs_str(d["test_ini"])})'
            cases.append(
                f'    Case {{ name: {rs_str(name + " " + t.plugin)}, table: &{m}::TABLE, image: {rs_str(img)}, base: {base:#x}, '
                f'test_ini: {ini}, addr: &[{", ".join(rs_str(a) for a in addr)}], '
                f'field: &[{", ".join(rs_str(a) for a in field_queries(d, t))}], note_for: &[{", ".join(rs_str(n) for n in nf)}], '
                f'content: {rs_str(content_expect(d, t) or "?")}, '
                f'expect: &[{", ".join(rs_str(x) for x in expected(d, t))}] }},')
    o += ['', 'pub const CASES: &[Case] = &['] + cases + ['];', '']
    return '\n'.join(o)


def run_rs(fx, tmp):
    out = os.path.join(tmp, 'fixtures.rs')
    with open(out, 'w') as f:
        f.write(rust_module(fx, tmp))
    cargo = os.path.expanduser('~/.cargo/bin/cargo')
    cargo = cargo if os.path.exists(cargo) else (shutil.which('cargo') or 'cargo')
    env = dict(os.environ, A2E_FIXTURES_RS=out)
    env.setdefault('CARGO_BUILD_JOBS', '2')
    r = subprocess.run([cargo, 'test', '--quiet', '--target', 'x86_64-unknown-linux-gnu', '--features', 'fixtures',
                        '--manifest-path', os.path.join(ROOT, 'crates/a2engine/Cargo.toml'), '--test', 'fixtures'],
                       env=env, capture_output=True, text=True)
    if r.returncode:
        print('FAIL rs:\n' + (r.stdout + r.stderr)[-4000:])
        return False
    return True


def bless(fx):
    for name, path, d, ts in fx:
        text = open(path).read()
        i = text.find('\n[expect]')
        if i >= 0:
            text = text[:i + 1]
        text = text.rstrip('\n') + '\n\n[expect]\n'
        for t in ts:
            text += f'{t.plugin} = [\n'
            for line in py_transcript(d, t):
                text += '  ' + rs_str(line) + ',\n'
            text += ']\n'
        with open(path, 'w') as f:
            f.write(text)
        print(f'blessed {os.path.relpath(path, ROOT)}')


def main(argv):
    fx = fixtures()
    if argv[:1] == ['bless']:
        bless(fx)
        return 0
    if argv[:1] == ['rust-module'] and len(argv) == 2:
        tmp = os.path.dirname(os.path.abspath(argv[1]))   # the images go beside it
        with open(argv[1], 'w') as f:
            f.write(rust_module(fx, tmp))
        return 0
    if argv[:1] != ['run']:
        print(__doc__, file=sys.stderr)
        return 2
    impls = argv[1:] or ['py', 'c', 'rs']
    tmp = tempfile.mkdtemp(prefix='a2e-fix-')
    try:
        ok = True
        n = sum(len(ts) for _, _, _, ts in fx)
        for impl in impls:
            r = {'py': lambda: run_py(fx), 'c': lambda: run_c(fx, tmp), 'rs': lambda: run_rs(fx, tmp)}[impl]()
            print(f'{impl}: {"ok" if r else "FAILED"} ({len(fx)} fixtures, {n} tables)')
            ok &= r
        return 0 if ok else 1
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
