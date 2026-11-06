#!/usr/bin/env python3
"""Host test of fo-scan.py on synthetic images, a synthetic map, log and profile set, all
made in a temp directory in code: nothing of the game is read or committed.

    fo_scan_test.py
"""
import importlib.util
import io
import os
import sys
import tempfile
import tomllib
import zlib
from contextlib import redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import mkpe  # noqa: E402
import pe  # noqa: E402

spec = importlib.util.spec_from_file_location('fo_scan', os.path.join(HERE, 'fo-scan.py'))
fo = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fo)

FAIL = []


def check(cond, what):
    print(('ok   ' if cond else 'FAIL ') + what)
    if not cond:
        FAIL.append(what)


SECS = [
    {'name': '.text', 'va': 0x1000, 'vsize': 0x2000, 'raw': 0x2000, 'flags': 'code'},
    {'name': '.rdata', 'va': 0x3000, 'vsize': 0x1000, 'raw': 0x1000, 'flags': 'data'},
    {'name': '.data', 'va': 0x4000, 'vsize': 0x1000, 'raw': 0x200, 'flags': 'data'},
]
PRO = '55 8b ec'
FN = {'FnAlpha': 0x401000, 'FnBeta': 0x401100, 'FnCallee': 0x401200, 'FnBody': 0x401300,
      'FnUnused': 0x401400, 'FnRt': 0x401500, 'FnCalleeRt': 0x401600}


def stock_img():
    return {'timestamp': 0x5eed0001, 'image_base': 0x400000, 'image_size': 0x6000, 'section': SECS,
            'bytes': [{'at': a, 'hex': PRO + ' 90 90 90 90 90 90 90 90 90 90 90 90 90'} for a in FN.values()]}


def roots_img():
    img = stock_img()
    img['image_size'] = 0x8000
    img['section'] = SECS + [{'name': '.new', 'va': 0x6000, 'vsize': 0x1000, 'raw': 0x200, 'flags': 'code'}]
    img['bytes'] = [dict(b) for b in img['bytes']] + [
        {'at': 0x401110, 'hex': 'ba e8 03 00 00'},        # a constant that reads as a call: an edit
        {'at': 0x401320, 'hex': '90 90'},                 # FnBody, beyond its entry
        {'at': 0x401410, 'hex': 'cc'},                    # FnUnused
        {'at': 0x406100, 'hex': 'c3'},
    ]
    img['jmp'] = [{'at': 0x401004, 'to': 0x406100},       # FnAlpha: a detour
                  {'at': 0x401200, 'to': 0x406100},       # FnCallee: entry taken over
                  {'at': 0x401300, 'to': 0x406100}]       # FnBody: entry and body
    return img


MAP = """ Address         Publics by Value              Rva+Base

""" + ''.join(f' 0001:{a - 0x401000:08x}       {n}      {a:08x} f   t.obj\n' for n, a in FN.items()) + \
    ' 0002:00000000       DataThing        00403000     t.obj\n'

LOG = """MemDiff.asi: Test.exe
at load: FleetOpsHook.dll is loaded
pass load t=1ms: 3 ranges, 12 bytes
diff .text 0x401600 0x401605 len 5 was [55 8b ec 90 90] now [e9 00 00 00 00] jmp 0x10000000 FleetOpsHook.dll+0x10
diff .text 0x401504 0x401509 len 5 was [90 90 90 90 90] now [e8 00 00 00 00] call 0x10000000 FleetOpsHook.dll+0x10
diff .data 0x404000 0x404004 len 4 was [00 00 00 00] now [00 00 00 10]
pass 1s t=1000ms: 2 ranges, 10 bytes
diff .text 0x401600 0x401605 len 5 was [55 8b ec 90 90] now [e9 00 00 00 00] jmp 0x10000000 FleetOpsHook.dll+0x10
diff .text 0x401504 0x401509 len 5 was [90 90 90 90 90] now [e8 00 00 00 00] call 0x10000000 FleetOpsHook.dll+0x10
pass 3s t=3000ms: unchanged since the last pass
"""


def profile_set(root, stock_identity):
    d = os.path.join(root, 'profiles', 'stk-1')
    os.makedirs(d)
    ts, size, fp = stock_identity
    open(os.path.join(d, 'build.toml'), 'w').write(f"""schema = 1
game = "tg"
build = "stk-1"
label = "Test stock"
exe = ["Test.exe"]
inherits = ""
content = "stock"
[pe]
timestamp = {ts:#x}
image_size = {size:#x}
sections = {fp:#x}
image_base = 0x400000
code = [0x401000, 0x403000]
""")
    syms = ['schema = 1', 'plugin = "p"', 'lang = "rust"', 'out = "-"', 'readme = "./README.md"',
            '[hook.h1]', 'doc = "x"', '[hook.h2]', 'doc = "x"']
    for i, (n, a) in enumerate(FN.items()):
        if n == 'FnUnused':
            continue
        syms += ['[[sym]]', f'id = "f_{n.lower()}"', f'symbol = "{n}"', 'kind = "fn"', f'addr = {a:#x}',
                 f'check = "{PRO}"', f'used_by = ["h{1 + i % 2}"]']
    open(os.path.join(d, 'p.toml'), 'w').write('\n'.join(syms) + '\n')
    return os.path.join(root, 'profiles')


def main():
    # unit: ranges and the log
    check(fo.runs_between(b'abcdefgh', b'aXcdXfgh') == [(1, 5)], 'a gap of up to three equal bytes joins two ranges')
    check(fo.runs_between(b'abcdefghij', b'aXcdefXXij') == [(1, 2), (6, 8)], 'distant ranges stay apart')
    rt = fo.parse_memdiff(LOG)
    check(len(rt['ranges']) == 3 and rt['loaded'] == {'FleetOpsHook.dll': True}, 'MemDiff.log: last pass plus load .data')
    check(rt['ranges'][0][3:] == ('jmp', 'FleetOpsHook.dll'), 'MemDiff.log: kind and module')

    with tempfile.TemporaryDirectory(prefix='fo-scan-test-') as tmp:
        sd = mkpe.build(stock_img())
        rd = mkpe.build(roots_img())
        se, re_ = pe.Exe(sd), pe.Exe(rd)
        ident = fo.identify(rd, os.path.join(tmp, 'Data', 'Test.exe'))
        check(ident['sections'][0]['crc32'] == zlib.crc32(rd[re_.sections[0][4]:re_.sections[0][4] + 0x2000]),
              'identity: a section\'s CRC-32')
        os.makedirs(os.path.join(tmp, 'Data'))
        open(os.path.join(tmp, 'Data', 'Test.exe'), 'wb').write(rd)
        open(os.path.join(tmp, 'stock.exe'), 'wb').write(sd)
        open(os.path.join(tmp, 'stock.map'), 'w').write(MAP)
        open(os.path.join(tmp, 'memdiff.log'), 'w').write(LOG)
        open(os.path.join(tmp, 'Roots_Download_Version.txt'), 'w').write('Version 9.9T1 (1 January 2030)')
        open(os.path.join(tmp, 'patches.tsv'), 'w').write('')
        pdir = profile_set(tmp, se.identity())
        os.environ['A2_DATA'] = os.path.join(tmp, 'data')
        cfgp = os.path.join(tmp, 'fo-scan.toml')
        open(cfgp, 'w').write('callees = ["FnCallee", "FnCalleeRt", "FnBody"]\n'
                              '[[disable]]\nhooks = ["p.h2"]\nwhy = "off"\n')
        argv = ['--exe', os.path.join(tmp, 'Data', 'Test.exe'), '--stock', os.path.join(tmp, 'stock.exe'),
                '--map', os.path.join(tmp, 'stock.map'), '--memdiff', os.path.join(tmp, 'memdiff.log'),
                '--patches', os.path.join(tmp, 'patches.tsv'), '--profiles', pdir, '--profile-game', 'tg',
                '--config', cfgp, '--report', os.path.join(tmp, 'report.md')]
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = fo.main(argv)
        check(rc == 0, 'scan runs')
        out = os.path.join(pdir, 'fo-roots-9.9T1')
        check(os.path.isfile(os.path.join(out, 'build.toml')), 'a new build is written into the profile set, named from the version')
        marks, checks, dis, p = fo.read_marks(out)
        check(marks == {'FnAlpha': 'detour', 'FnBeta': 'edit', 'FnBody': 'detour', 'FnRt': 'runtime'},
              f'marks: detour, edit (a constant is no call), a callee beyond its entry, runtime: {marks}')
        check(checks == {('p.toml', 'f_fncallee'), ('p.toml', 'f_fncalleert')}, f'callees drop their check: {checks}')
        check(dis == {'p.h2'}, 'disable from the config')
        check(p['image_size'] == 0x8000 and p['code'] == [0x401000, 0x403000], '[pe] identity and code range')
        games = fo.profiles.load_game_sets([pdir])
        check([b.name for b in games['tg'].builds] == ['stk-1', 'fo-roots-9.9T1'], 'the profile set loads')
        rep = open(os.path.join(tmp, 'report.md')).read()
        check('FnUnused' in rep and '## Marks' in rep, 'the report names the changed function no entry uses')
        check('FnCallee' not in open(os.path.join(out, 'build.toml')).read(), 'no callee in build.toml')

        # again: the profile exists, so the scan goes beside the report and is compared
        buf = io.StringIO()
        with redirect_stdout(buf):
            fo.main(argv)
        check('the same marks' in buf.getvalue(), 're-running compares with the existing profile: the same marks')
        text = open(os.path.join(out, 'build.toml')).read().replace('how = "edit"', 'how = "detour"')
        open(os.path.join(out, 'build.toml'), 'w').write(text)
        buf = io.StringIO()
        with redirect_stdout(buf):
            fo.main(argv)
        check('FnBeta: scan says edit' in buf.getvalue(), 'a changed mark is reported')
        check(tomllib.loads(open(os.path.join(out, 'build.toml')).read())['build'] == 'fo-roots-9.9T1', 'build.toml parses')

    print('fo-scan test: ' + ('FAILED' if FAIL else 'ok'))
    return 1 if FAIL else 0


if __name__ == '__main__':
    sys.exit(main())
